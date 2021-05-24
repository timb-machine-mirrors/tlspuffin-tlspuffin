use core::time::Duration;
use std::marker::PhantomData;
use std::path::Path;
use std::{env, fs, path::PathBuf, time, thread};

use libafl::bolts::rands::{Rand, RomuTrioRand};
use libafl::events::{Event, EventManager, LogSeverity};
use libafl::executors::Executor;
use libafl::feedbacks::{
    FeedbackStatesTuple, MapIndexesMetadata, MaxReducer, OrFeedback,
};
use libafl::inputs::{Input};
use libafl::{
    bolts::tuples::{tuple_list, Merge},
    bolts::{current_nanos, rands::StdRand},
    corpus::{
        Corpus, InMemoryCorpus, IndexesLenTimeMinimizerCorpusScheduler, OnDiskCorpus,
        QueueCorpusScheduler,
    },
    events::{setup_restarting_mgr_std, EventRestarter},
    executors::{inprocess::InProcessExecutor, ExitKind, TimeoutExecutor},
    feedback_or,
    feedbacks::{CrashFeedback, MapFeedbackState, MaxMapFeedback, TimeFeedback, TimeoutFeedback},
    fuzzer::{Fuzzer, StdFuzzer},
    mutators::scheduled::{tokens_mutations, StdScheduledMutator},
    mutators::token_mutations::Tokens,
    observers::{HitcountsMapObserver, StdMapObserver, TimeObserver},
    stages::mutational::StdMutationalStage,
    state::{HasCorpus, HasMetadata, StdState},
    stats::SimpleStats,
    Error, Evaluator,
};
/*use libafl_targets::{EDGES_MAP, MAX_EDGES_NUM};*/

use crate::fuzzer::mutations::{trace_mutations};
use crate::trace::Trace;
use rand::Rng;
use libafl::corpus::RandCorpusScheduler;

mod mutations;

pub fn start_fuzzing() {
    // Registry the metadata types used in this fuzzer
    // Needed only on no_std
    //RegistryBuilder::register::<Tokens>();

    println!(
        "Workdir: {:?}",
        env::current_dir().unwrap().to_string_lossy().to_string()
    );
    fuzz(
        &[PathBuf::from("./corpus")],
        PathBuf::from("./crashes"),
        1337,
    )
    .expect("An error occurred while fuzzing");
}

fn harness(input: &Trace) -> ExitKind {
    let mut rng = rand::thread_rng();

    let n1 = rng.gen_range(0..10);
    println!("Run {}", n1);
    if n1 <= 3 {
        panic!()
    }
    let ten_millis = time::Duration::from_millis(1000);

    thread::sleep(ten_millis);
    ExitKind::Timeout
}

/// The actual fuzzer
fn fuzz(corpus_dirs: &[PathBuf], objective_dir: PathBuf, broker_port: u16) -> Result<(), Error> {
    // 'While the stats are state, they are usually used in the broker - which is likely never restarted
    let stats = SimpleStats::new(|s| println!("{}", s));

    // The restarting state will spawn the same process again as child, then restarted it each time it crashes.
    let (state, mut restarting_mgr) = match setup_restarting_mgr_std(stats, broker_port) {
        Ok(res) => res,
        Err(err) => match err {
            Error::ShuttingDown => {
                return Ok(());
            }
            _ => {
                panic!("Failed to setup the restarter: {}", err);
            }
        },
    };


    // Create an observation channel to keep track of the execution time
    let time_observer = TimeObserver::new("time");

    // Feedback to rate the interestingness of an input
    // This one is composed by two Feedbacks in OR
    let feedback = feedback_or!(
        // Time feedback, this one does not need a feedback state
        TimeFeedback::new_with_observer(&time_observer),
        TimeoutFeedback::new()
    );

    // A feedback to choose if an input is a solution or not
    let objective = feedback_or!(CrashFeedback::new(), TimeoutFeedback::new());

    // If not restarting, create a State from scratch
    let mut state = state.unwrap_or_else(|| {
        StdState::new(
            // RNG
            StdRand::with_seed(current_nanos()),
            // Corpus that will be evolved, we keep it in memory for performance
            InMemoryCorpus::new(),
            // Corpus in which we store solutions (crashes in this example),
            // on disk so the user can get them after stopping the fuzzer
            OnDiskCorpus::new(objective_dir).unwrap(),
            // States of the feedbacks.
            // They are the data related to the feedbacks that you want to persist in the State.
            tuple_list!(),
        )
    });

    //let mut rand: Box<dyn HasRand<_>> = Box::new(state);
    //let mut rand: Box<dyn HasRand<RomuTrioRand>> = Box::new(state);
    //rand.rand();

    println!("We're a client, let's fuzz :)");

    // Setup a basic mutator with a mutational stage
    let mutator = StdScheduledMutator::new(trace_mutations());
    let mut stages = tuple_list!(StdMutationalStage::new(mutator));

    // A minimization+queue policy to get testcasess from the corpus
    // let scheduler = IndexesLenTimeMinimizerCorpusScheduler::new(QueueCorpusScheduler::new());
    let scheduler = RandCorpusScheduler::new();

    // A fuzzer with feedbacks and a corpus scheduler
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    // Create the executor for an in-process function with one observer for edge coverage and one for the execution time
    let harness_fn = &mut harness;
    let mut executor = TimeoutExecutor::new(
        InProcessExecutor::new(
            harness_fn,
            tuple_list!(time_observer),
            &mut fuzzer,
            &mut state,
            &mut restarting_mgr,
        )?,
        // 10 seconds timeout
        Duration::new(10, 0),
    );

    // In case the corpus is empty (on first run), reset
    if state.corpus().count() < 1 {
        load_initial_inputs(
            &mut state,
            &mut fuzzer,
            &mut executor,
            &mut restarting_mgr,
            &|path: &Path| {
                let bytes = fs::read(path).unwrap();
                serde_json::from_slice::<Trace>(bytes.as_slice()).unwrap()
            },
            &corpus_dirs,
        )
        .unwrap_or_else(|err| panic!("Failed to load initial corpus at {:?}: {}", &corpus_dirs, err));
        println!("We imported {} inputs from disk.", state.corpus().count());
    }

    // This fuzzer restarts after 1 mio `fuzz_one` executions.
    // Each fuzz_one will internally do many executions of the target.
    // If your target is very instable, setting a low count here may help.
    // However, you will lose a lot of performance that way.
    let iters = 1_000_000;
    fuzzer.fuzz_loop_for(
        &mut stages,
        &mut state,
        &mut executor,
        &mut restarting_mgr,
        iters,
    )?;


    // It's important, that we store the state before restarting!
    // Else, the parent will not respawn a new child and quit.
    restarting_mgr.on_restart(&mut state)?;

    Ok(())
}

// todo Needs upstreaming:

pub fn load_initial_inputs<E, EM, Z, C, FT, I, R, SC>(
    state: &mut StdState<C, FT, I, R, SC>,
    fuzzer: &mut Z,
    executor: &mut E,
    manager: &mut EM,
    reader: &dyn Fn(&Path) -> I,
    in_dirs: &[PathBuf],
) -> Result<(), Error>
where
    Z: Evaluator<E, EM, I, StdState<C, FT, I, R, SC>>,
    EM: EventManager<E, I, StdState<C, FT, I, R, SC>, Z>,
    I: Input,
    C: Corpus<I>,
    R: Rand,
    FT: FeedbackStatesTuple,
    SC: Corpus<I>,
    E: Executor<I>,
{
    for in_dir in in_dirs {
        load_from_directory(state, fuzzer, executor, manager, reader, in_dir)?;
    }
    manager.fire(
        state,
        Event::Log {
            severity_level: LogSeverity::Debug,
            message: format!("Loaded {} initial testcases.", state.corpus().count()), // get corpus count
            phantom: PhantomData,
        },
    )?;
    manager.process(fuzzer, state, executor)?;
    Ok(())
}

/// loads inputs from a directory
fn load_from_directory<E, EM, Z, C, FT, I, R, SC>(
    state: &mut StdState<C, FT, I, R, SC>,
    fuzzer: &mut Z,
    executor: &mut E,
    manager: &mut EM,
    reader: &dyn Fn(&Path) -> I,
    in_dir: &Path,
) -> Result<(), Error>
where
    Z: Evaluator<E, EM, I, StdState<C, FT, I, R, SC>>,
    I: Input,
    C: Corpus<I>,
    R: Rand,
    FT: FeedbackStatesTuple,
    SC: Corpus<I>,
    E: Executor<I>,
{
    for entry in fs::read_dir(in_dir)? {
        let entry = entry?;
        let path = entry.path();
        let attributes = fs::metadata(&path);

        if attributes.is_err() {
            continue;
        }

        let attr = attributes?;

        if attr.is_file() && attr.len() > 0 {
            println!("Loading file {:?} ...", &path);
            let input = reader(&path);
            let (is_interesting, _) = fuzzer.evaluate_input(state, executor, manager, input)?;
            if !is_interesting {
                println!("File {:?} was not interesting, skipped.", &path);
            }
        } else if attr.is_dir() {
            load_from_directory(state, fuzzer, executor, manager, reader, &path)?;
        }
    }

    Ok(())
}