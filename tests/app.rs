use nedit::app::{App, NotificationType};

#[test]
fn test_app_initialization() {
    let args = vec![];
    let app = App::new(&args);
    
    assert!(app.is_welcome);
    assert!(!app.should_quit);
    assert_eq!(app.buffers.len(), 0);
}

#[test]
fn test_app_notifications() {
    let mut app = App::new(&[]);
    
    app.show_notification("Test Msg".to_string(), NotificationType::Info);
    
    if let Some((msg, ntype)) = &app.notification {
        assert_eq!(msg, "Test Msg");
        assert!(matches!(ntype, NotificationType::Info));
    } else {
        panic!("Notification should be set");
    }
    
    app.clear_notification();
    assert!(app.notification.is_none());
}

#[test]
fn test_app_notification_tick() {
    let mut app = App::new(&[]);
    app.show_notification("Tick Test".to_string(), NotificationType::Info);
    
    // Initial timer is 5
    for _ in 0..5 {
        assert!(app.notification.is_some());
        app.tick_notification();
    }
    
    assert!(app.notification.is_none());
}
