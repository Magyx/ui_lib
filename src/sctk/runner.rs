use std::{any::Any, cell::RefCell, collections::VecDeque, rc::Rc};

use crate::{
    context::MessageSink,
    engine::TargetId,
    task::{BoxWork, Payload, TaskId, TaskRunner},
};

#[derive(Clone, Default)]
pub struct SctkMessageSink(Rc<RefCell<Vec<Box<dyn Any>>>>);
impl MessageSink for SctkMessageSink {
    fn emit(&mut self, msg: Box<dyn Any>) {
        self.0.borrow_mut().push(msg);
    }
    fn drain(&mut self) -> Vec<Box<dyn Any>> {
        std::mem::take(&mut *self.0.borrow_mut())
    }
}
impl SctkMessageSink {
    pub(super) fn push(&mut self, msg: Box<dyn Any>) {
        self.0.borrow_mut().push(msg);
    }
}

pub struct CalloopRunner {
    tx: calloop::channel::Sender<(TargetId, TaskId, Payload)>,
    inbox: Rc<RefCell<VecDeque<(TargetId, TaskId, Payload)>>>,
}
impl CalloopRunner {
    pub fn new() -> (Self, calloop::channel::Channel<(TargetId, TaskId, Payload)>) {
        let (tx, rx) = calloop::channel::channel();
        let inbox = Rc::new(RefCell::new(VecDeque::new()));
        (Self { tx, inbox }, rx)
    }

    pub fn inbox(&self) -> Rc<RefCell<VecDeque<(TargetId, TaskId, Payload)>>> {
        self.inbox.clone()
    }
}
impl TaskRunner for CalloopRunner {
    fn spawn(&self, target: TargetId, id: TaskId, run: BoxWork) {
        let tx = self.tx.clone();
        std::thread::Builder::new()
            .name("ui-task".into())
            .spawn(move || {
                let payload = pollster::block_on(run);
                // Sending wakes the loop; an error means the loop/receiver is
                // gone (shutting down), so dropping the payload is correct.
                let _ = tx.send((target, id, payload));
            })
            .expect("spawn ui-task thread");
    }

    fn drain(&self, out: &mut Vec<(TargetId, TaskId, Payload)>) {
        out.extend(self.inbox.borrow_mut().drain(..));
    }
}
