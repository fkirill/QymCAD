//! A LONG SETTLE THROUGH THE PUMP keeps the live frames coming: the agent's work runs a slice to a live
//! frame, and no frame of the person's window is held still while the agent waits the program out.
#[cfg(test)]
mod tests {
    use super::super::App;
    use std::time::Duration;

    /// THE LIVE FRAMES NEVER STALL UNDER A LONG SETTLE: a program busy for half a second - its greeting on
    /// screen - is a settle no single slice can outwait. The call's work settles in slices of five
    /// milliseconds; each call of the pump takes one slice and hands the frame back, and the answer comes
    /// only when the whole settle has run. What is measured is the live loop itself: many frames, none of
    /// them long.
    #[test]
    fn the_live_frames_never_stall_under_a_long_settle() {
        let mut app = App::default();
        let ctx = egui::Context::default();
        app.waiting.splash_until = Some(std::time::Instant::now() + Duration::from_millis(500));
        let (answer, told) = std::sync::mpsc::channel();
        let work: Box<crate::mcp::Work> = Box::new(|core| core.settle_for(Duration::from_millis(5)));
        crate::mcp::hand_over(work, answer);
        let began = std::time::Instant::now();
        let (mut live_frames, mut longest) = (0, Duration::ZERO);
        while told.try_recv().is_err() {
            let frame_began = std::time::Instant::now();
            crate::mcp::pump(&mut app, &ctx);
            longest = longest.max(frame_began.elapsed());
            live_frames += 1;
            assert!(began.elapsed() < Duration::from_secs(10), "the settle never came to rest");
        }
        assert!(live_frames > 5, "a settle of half a second ran in {live_frames} frames: the live window was held still");
        assert!(longest < Duration::from_millis(250), "one live frame was held {longest:?} for the agent's sake");
    }
}
