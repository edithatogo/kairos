use kairo_ecs_types::SimTime;
use kairo_ecs_viz::{renderer_statuses, RenderFrame, RendererStatus};

#[test]
fn renderer_feature_state_is_explicit() {
    let statuses = renderer_statuses();

    #[cfg(not(any(feature = "wgpu-renderer", feature = "bevy-renderer")))]
    assert_eq!(statuses, vec![RendererStatus::HeadlessOnly]);

    #[cfg(feature = "wgpu-renderer")]
    assert!(statuses.contains(&RendererStatus::WgpuFeatureEnabledButNotConfigured));

    #[cfg(feature = "bevy-renderer")]
    assert!(statuses.contains(&RendererStatus::BevyFeatureEnabledButNotConfigured));
}

#[test]
fn empty_headless_frame_is_valid() {
    let frame = RenderFrame::new(SimTime::ZERO);
    let summary = kairo_ecs_viz::render_headless(&frame).expect("headless render");

    assert_eq!(summary.entity_count, 0);
    assert_eq!(summary.event_count, 0);
    assert_eq!(summary.bounds, None);
}
