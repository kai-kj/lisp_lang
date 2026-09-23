use crate::{
    frontend::event::{Event, EventEmitter, EventError, SEvent, SEventError},
    runtime::{session::Session, value::Value},
    span::{Span, SpannedExt},
};

pub struct ValueReader {
    events: Vec<SEvent>,
    pos: usize,
}

impl EventEmitter for ValueReader {
    fn peek(&self) -> Result<&SEvent, SEventError> {
        Ok(&self.events[self.pos])
    }

    fn next(&mut self) -> Result<SEvent, SEventError> {
        let event = self.events[self.pos].clone();
        if self.pos + 1 < self.events.len() {
            self.pos += 1;
        }
        Ok(event)
    }
}

impl ValueReader {
    pub fn new(session: &Session, value: Value, span: Span) -> Result<Self, SEventError> {
        let mut events = Vec::new();
        Self::emit(&value, session, span, &mut events)?;
        events.push(Event::SourceEnd.scopy(span));
        Ok(Self { events, pos: 0 })
    }

    fn emit(
        value: &Value,
        session: &Session,
        span: Span,
        events: &mut Vec<SEvent>,
    ) -> Result<(), SEventError> {
        let event = match value {
            Value::Null => {
                events.push(Event::Symbol("null".to_string()).scopy(span));
                return Ok(());
            }
            Value::List(values) => {
                events.push(Event::ListStart.scopy(span));
                for value in values.iter() {
                    Self::emit(value, session, span, events)?;
                }
                events.push(Event::ListEnd.scopy(span));
                return Ok(());
            }
            Value::Symbol(symbol) => Event::Symbol(session.get_symbol(*symbol).to_owned()),
            Value::Boolean(true) => Event::Symbol("true".to_string()),
            Value::Boolean(false) => Event::Symbol("false".to_string()),
            Value::Integer(value) => Event::Integer(*value),
            Value::Float(value) => Event::Float(*value),
            Value::String(value) => Event::String(value.as_ref().clone()),
            Value::BuiltinFn(_) | Value::UserFn(_) => {
                return Err(EventError::UnexpectedValue.scopy(span));
            }
        };
        events.push(event.scopy(span));
        Ok(())
    }
}
