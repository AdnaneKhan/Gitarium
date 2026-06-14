//! A minimal `join_all`: drive a set of futures concurrently to completion,
//! returning their outputs in input order. Stand-in for
//! `futures::future::join_all` (no such dependency here); each poll advances
//! every still-pending child. Bound the input size by the caller — it polls
//! all entries it's given.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(super) fn join_all<F: Future>(futs: impl IntoIterator<Item = F>) -> JoinAll<F> {
    let futs: Vec<_> = futs.into_iter().map(|f| Some(Box::pin(f))).collect();
    let out = futs.iter().map(|_| None).collect();
    JoinAll { futs, out }
}

pub(super) struct JoinAll<F: Future> {
    futs: Vec<Option<Pin<Box<F>>>>,
    out: Vec<Option<F::Output>>,
}

// The futures are heap-pinned in `Box`es (never moved) and only finished
// outputs are moved out, so JoinAll is safe to treat as Unpin for any `F`.
impl<F: Future> Unpin for JoinAll<F> {}

impl<F: Future> Future for JoinAll<F> {
    type Output = Vec<F::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();
        let mut pending = false;
        for (slot, out) in me.futs.iter_mut().zip(me.out.iter_mut()) {
            if let Some(fut) = slot {
                match fut.as_mut().poll(cx) {
                    Poll::Ready(v) => {
                        *out = Some(v);
                        *slot = None;
                    }
                    Poll::Pending => pending = true,
                }
            }
        }
        if pending {
            Poll::Pending
        } else {
            Poll::Ready(me.out.iter_mut().map(|o| o.take().unwrap()).collect())
        }
    }
}
