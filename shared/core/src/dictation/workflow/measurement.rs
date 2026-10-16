//! Observed monotonic boundaries, never estimates of model/server work.
use std::time::Instant;

#[derive(Clone, Copy)]
pub struct Origin {
    pub released: Instant,
    pub audio_millis: u64,
}
pub struct Measurement {
    pub stages: [u64; 5],
    pub audio_millis: u64,
    pub input_chars: usize,
    pub input_words: u64,
    pub output_chars: usize,
    pub sent_chars: usize,
}
impl Measurement {
    pub fn full_millis(&self) -> u64 {self.stages.iter().sum()}
    pub(crate) fn observed(origin: Origin, offsets: [u64; 5], input_chars: usize, input_words: u64, sent_chars: usize, output_chars: usize) -> Self {
        let mut previous=0;
        let stages=offsets.map(|value| {let next=value.max(previous);let elapsed=next-previous;previous=next;elapsed});
        Self {stages,audio_millis:origin.audio_millis,input_chars,input_words,sent_chars,output_chars}
    }
    pub fn trace(&self, trace: Option<String>, own: bool, failed: bool, status: Option<u16>, inserted: bool) -> String {
        let result=if failed {if status.is_some(){"http_error"}else if own {"server_error"}else{"local_error"}} else {"success"};
        let mut trace=trace.unwrap_or_else(||format!("timing_v1 phase={} result={result} elapsed_ms={} deadline_ms=0 local_ms=0 start_ms=0 wait_ms=0 finalize_ms=0 starts=0 polls=0 transport_errors=0 transport_timeouts=0 http_errors={}",
            if own {"finalize"} else {"local"},if own {self.stages[3]} else {0},u8::from(status.is_some())));
        if own {trace.push_str(" self_hosted=1");if let Some(status)=status {trace.push_str(&format!(" last_http={status}"));}}
        if !inserted || failed {trace=trace.replace("result=success","result=local_error");}
        trace.push_str(&format!(" full_ms={} tail_ms={} recognition_ms={} delivery_prep_ms={} processing_ms={} insertion_ms={} audio_ms={} offline_chars={} offline_words={} output_chars={} sent_chars={}",
            self.full_millis(),self.stages[0],self.stages[1],self.stages[2],self.stages[3],self.stages[4],self.audio_millis,self.input_chars,self.input_words,self.output_chars,self.sent_chars));
        trace
    }
}
pub(crate) fn offset(origin: Origin) -> u64 {origin.released.elapsed().as_millis().min(u64::MAX as u128) as u64}
