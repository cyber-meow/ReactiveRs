use runtime::Runtime;
use continuation::Continuation;

/// A continuation that applies a function before calling another continuation.
pub struct Map<C, F> { pub(crate) continuation: C, pub(crate) map: F }

impl<R, C, F, V1, V2> Continuation<R, V1> for Map<C, F>
    where R: Runtime, C: Continuation<R, V2>, F: FnOnce(V1) -> V2 + 'static
{
    fn call(self, runtime: &mut R, value: V1) {
        let v2 = (self.map)(value);
        self.continuation.call(runtime, v2);
    }

    fn call_box(self: Box<Self>, runtime: &mut R, value: V1) {
        (*self).call(runtime, value);
    }
}
