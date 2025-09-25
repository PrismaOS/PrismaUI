/// Premium animations for PrismaUI components
use gpui::Animation;
use std::time::Duration;
use std::time::Duration as StdDuration;

pub struct PremiumAnimations;

impl PremiumAnimations {
    /// Modal appear animation - smooth fade and slide up
    pub fn modal_appear() -> Animation {
        Animation::new(StdDuration::from_secs_f64(0.25))
    }

    /// Panel slide up animation - quick slide from bottom
    pub fn panel_slide_up() -> Animation {
        Animation::new(StdDuration::from_secs_f64(0.2))
    }
}
