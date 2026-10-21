//! Ordered observable output shared by future text and JSON presentations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Stdout,
    Event,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub channel: Channel,
    pub text: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Log {
    observations: Vec<Observation>,
    bytes: u64,
    reserved: u64,
}
impl Log {
    pub fn observations(&self) -> &[Observation] {
        &self.observations
    }
    pub fn stdout(&self) -> Vec<&str> {
        self.observations
            .iter()
            .filter(|o| o.channel == Channel::Stdout)
            .map(|o| o.text.as_str())
            .collect()
    }
    pub fn events(&self) -> Vec<&str> {
        self.observations
            .iter()
            .filter(|o| o.channel == Channel::Event)
            .map(|o| o.text.as_str())
            .collect()
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
    pub fn reserved_bytes(&self) -> u64 {
        self.reserved
    }
    pub fn push_stdout(&mut self, text: String) -> Result<(), u64> {
        self.push(Channel::Stdout, text, "")
    }
    pub fn push_event(&mut self, text: String, prefix: &str) -> Result<(), u64> {
        self.push(Channel::Event, text, prefix)
    }
    fn push(&mut self, channel: Channel, text: String, prefix: &str) -> Result<(), u64> {
        let n = u64::try_from(prefix.len() + text.len() + 1).map_err(|_| u64::MAX)?;
        let proposed = self
            .bytes
            .checked_add(self.reserved)
            .and_then(|v| v.checked_add(n))
            .ok_or(u64::MAX)?;
        if proposed > 65_536 {
            return Err(proposed);
        }
        self.bytes += n;
        self.observations.push(Observation { channel, text });
        Ok(())
    }
    pub fn reserve(&mut self, bytes: u64) -> Result<(), u64> {
        let proposed = self
            .bytes
            .checked_add(self.reserved)
            .and_then(|v| v.checked_add(bytes))
            .ok_or(u64::MAX)?;
        if proposed > 65_536 {
            return Err(proposed);
        }
        self.reserved += bytes;
        Ok(())
    }
    pub fn emit_reserved_event(&mut self, text: String, prefix: &str) -> Result<(), &'static str> {
        let n = u64::try_from(prefix.len() + text.len() + 1).map_err(|_| "event length")?;
        if self.reserved < n {
            return Err("missing reservation");
        }
        self.reserved -= n;
        self.bytes = self.bytes.checked_add(n).ok_or("event bytes")?;
        self.observations.push(Observation {
            channel: Channel::Event,
            text,
        });
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_bytes_include_lf_and_rejected_line_is_atomic() {
        let mut log = Log::default();
        log.push_stdout("25".into()).unwrap();
        assert_eq!(log.bytes(), 3);
        let mut exact = Log::default();
        exact.push_stdout("x".repeat(65_535)).unwrap();
        assert_eq!(exact.bytes(), 65_536);
        assert_eq!(exact.push_stdout(String::new()), Err(65_537));
        assert_eq!(log.push_stdout("x".repeat(65_533)), Err(65_537));
        assert_eq!(log.stdout(), vec!["25"]);
        assert_eq!(log.bytes(), 3);
    }
    #[test]
    fn reserved_closure_converts_bytes_without_growth() {
        let mut log = Log::default();
        log.reserve(24).unwrap();
        log.push_event("open:0:7".into(), "mock-resource:").unwrap();
        assert_eq!((log.bytes(), log.reserved_bytes()), (23, 24));
        log.emit_reserved_event("close:0:7".into(), "mock-resource:")
            .unwrap();
        assert_eq!((log.bytes(), log.reserved_bytes()), (47, 0));
        assert_eq!(log.events(), vec!["open:0:7", "close:0:7"]);
    }
}
