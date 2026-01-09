use risten::routing::{HashMapRouterBuilder, RouteResult, Router, RouterBuilder};

#[cfg(feature = "matchit")]
use risten::source::router::MatchitRouterBuilder;

#[cfg(feature = "phf")]
use risten::source::router::PhfRouter;

#[cfg(feature = "matchit")]
#[test]
fn test_matchit_router() {
    let mut builder = MatchitRouterBuilder::default();
    builder.insert("/events/{type}".to_string(), 1).unwrap();
    builder.insert("/system/{*rest}".to_string(), 2).unwrap();
    let router = builder.build().unwrap();

    // Test parameter match
    match router.route("/events/login") {
        RouteResult::Matched(val) => assert_eq!(*val, 1),
        RouteResult::NotFound => panic!("Should match /events/login"),
    }

    // Test wildcard match
    match router.route("/system/error/critical") {
        RouteResult::Matched(val) => assert_eq!(*val, 2),
        RouteResult::NotFound => panic!("Should match /system/error/critical"),
    }

    // Test no match
    match router.route("/files/image.png") {
        RouteResult::Matched(_) => panic!("Should not match"),
        RouteResult::NotFound => {}
    }
}

#[cfg(feature = "phf")]
#[test]
fn test_phf_router() {
    use phf::phf_map;

    static MY_MAP: phf::Map<&'static str, i32> = phf_map! {
        "home" => 1,
        "about" => 2,
    };

    let router = PhfRouter::new(&MY_MAP);

    assert_eq!(router.route("home"), RouteResult::Matched(&1));
    assert_eq!(router.route("about"), RouteResult::Matched(&2));
    assert_eq!(router.route("contact"), RouteResult::NotFound);
}

#[test]
fn test_hashmap_router() {
    let mut builder = HashMapRouterBuilder::default();
    builder.insert("home".to_string(), 1).unwrap();
    builder.insert("about".to_string(), 2).unwrap();
    let router = builder.build().unwrap();

    // HashMapRouter uses String keys, so we match against string slices
    assert_eq!(router.route(&"home".to_string()), RouteResult::Matched(&1));
    assert_eq!(router.route(&"about".to_string()), RouteResult::Matched(&2));
    assert_eq!(router.route(&"contact".to_string()), RouteResult::NotFound);
}
