//! Temporary module for a PIKO site canister: sends every cycle above KEEP
//! to TARGET (mother by default) once, right after install, then does
//! nothing. The site's own module (piko-site) is reinstalled right after.
//!
//! - No update method, every ingress update refused.
//! - The target is fixed at build time (SWEEP_TARGET, default mother), so
//!   the module hash says where the cycles go.
//! - `sweep_status` (query) reports the outcome.

use candid::{CandidType, Principal};
use std::cell::RefCell;
use std::time::Duration;

const KEEP: u128 = 30_000_000_000_000; // 30T stay on the site

fn target() -> Principal {
    Principal::from_text(option_env!("SWEEP_TARGET").unwrap_or("45mjf-rqaaa-aaaaj-qsedq-cai")).unwrap()
}

#[derive(CandidType)]
struct DepositCyclesArgs {
    canister_id: Principal,
}

thread_local! {
    static STATUS: RefCell<String> = RefCell::new("pending".to_string());
}

fn set(s: String) {
    STATUS.with_borrow_mut(|v| *v = s);
}

async fn sweep() {
    let balance = ic_cdk::api::canister_cycle_balance();
    if balance <= KEEP {
        set(format!("nothing to send: balance {balance} <= keep {KEEP}"));
        return;
    }
    let amount = balance - KEEP;
    let to = target();
    let r = ic_cdk::call::Call::unbounded_wait(Principal::management_canister(), "deposit_cycles")
        .with_arg(DepositCyclesArgs { canister_id: to })
        .with_cycles(amount)
        .await;
    match r {
        Ok(_) => set(format!("sent {amount} to {to}; balance now {}", ic_cdk::api::canister_cycle_balance())),
        Err(e) => set(format!("FAILED sending {amount} to {to}: {e:?}")),
    }
}

#[ic_cdk::init]
fn init() {
    ic_cdk_timers::set_timer(Duration::ZERO, sweep());
}

#[ic_cdk::inspect_message]
fn inspect_message() {}

#[ic_cdk::query]
fn sweep_status() -> String {
    STATUS.with_borrow(|s| s.clone())
}

ic_cdk::export_candid!();
