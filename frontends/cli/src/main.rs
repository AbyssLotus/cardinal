//! # Cardinal CLI frontend
//!
//! A consumer at the frontend seam (Vol. V Ch. 1; Vol. V Ch. 10 §10.1). It loads a world
//! through the `packages` service and reads entitled streams from `observe`, submitting
//! validated actions through the front door -- it never reaches into the tick's interior
//! (Vol. V Ch. 10 §10.1: no sixth seam).

fn main() {
    // Scaffold entry point. Building and running this proves the full dependency chain
    // frontend -> services -> kernel links under the layer law (Vol. V Ch. 1 §1.1).
    println!(
        "Cardinal reference engine -- workspace scaffold.\n\
         Spec-first: build order is kernel, then the determinism harness, then domains \
         (Vol. V Ch. 10 §10.4)."
    );
}
