Hey qwen so on my github actions workflow this failed on mac using the latest beta channel for rust with the following errors

failures:

---- unit::core::test_session::test_session_manager_list_sessions_default stdout ----

thread 'unit::core::test_session::test_session_manager_list_sessions_default' (37198) panicked at tests/unit/core/test_session.rs:228:5:
assertion failed: sessions.contains(&"default".to_string())
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/1b6e21e163baa0b20f119e17e3871910978a60b6/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/1b6e21e163baa0b20f119e17e3871910978a60b6/library/core/src/panicking.rs:80:14
   2: core::panicking::panic
             at /rustc/1b6e21e163baa0b20f119e17e3871910978a60b6/library/core/src/panicking.rs:150:5
   3: main::unit::core::test_session::test_session_manager_list_sessions_default
             at ./tests/unit/core/test_session.rs:228:5
   4: main::unit::core::test_session::test_session_manager_list_sessions_default::{{closure}}
             at ./tests/unit/core/test_session.rs:199:48
   5: core::ops::function::FnOnce::call_once
             at /rustc/1b6e21e163baa0b20f119e17e3871910978a60b6/library/core/src/ops/function.rs:250:5
   6: core::ops::function::FnOnce::call_once
             at /rustc/1b6e21e163baa0b20f119e17e3871910978a60b6/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- unit::core::test_session::test_session_manager_list_sessions_named stdout ----

thread 'unit::core::test_session::test_session_manager_list_sessions_named' (37199) panicked at tests/unit/core/test_session.rs:236:35:
called `Result::unwrap()` on an `Err` value: PoisonError { .. }
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/1b6e21e163baa0b20f119e17e3871910978a60b6/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/1b6e21e163baa0b20f119e17e3871910978a60b6/library/core/src/panicking.rs:80:14
   2: core::result::unwrap_failed
             at /rustc/1b6e21e163baa0b20f119e17e3871910978a60b6/library/core/src/result.rs:1867:5
   3: core::result::Result<T,E>::unwrap
             at /rustc/1b6e21e163baa0b20f119e17e3871910978a60b6/library/core/src/result.rs:1233:23
   4: main::unit::core::test_session::test_session_manager_list_sessions_named
             at ./tests/unit/core/test_session.rs:236:35
   5: main::unit::core::test_session::test_session_manager_list_sessions_named::{{closure}}
             at ./tests/unit/core/test_session.rs:235:46
   6: core::ops::function::FnOnce::call_once
             at /rustc/1b6e21e163baa0b20f119e17e3871910978a60b6/library/core/src/ops/function.rs:250:5
   7: core::ops::function::FnOnce::call_once
             at /rustc/1b6e21e163baa0b20f119e17e3871910978a60b6/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    unit::core::test_session::test_session_manager_list_sessions_default
    unit::core::test_session::test_session_manager_list_sessions_named

test result: FAILED. 167 passed; 2 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.89s

Error: Process completed with exit code 101.