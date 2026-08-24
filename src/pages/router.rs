#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Route {
    #[default]
    Home,
    About,
    Options,
}

#[derive(Debug, Default)]
pub struct Router {
    current: Route,
}

impl Router {
    pub fn current(&self) -> Route {
        self.current
    }

    pub fn navigate(&mut self, route: Route) -> bool {
        if self.current == route {
            return false;
        }
        self.current = route;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_starts_at_home_and_changes_routes() {
        let mut router = Router::default();
        assert_eq!(router.current(), Route::Home);
        assert!(router.navigate(Route::About));
        assert_eq!(router.current(), Route::About);
        assert!(!router.navigate(Route::About));
    }
}
