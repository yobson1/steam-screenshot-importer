use gpui::ScrollHandle;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Route {
    #[default]
    Home,
    About,
    Options,
}

impl Route {
    const fn index(self) -> usize {
        match self {
            Self::Home => 0,
            Self::About => 1,
            Self::Options => 2,
        }
    }
}

#[derive(Debug)]
pub struct Router {
    current: Route,
    scroll_handles: [ScrollHandle; 3],
}

impl Default for Router {
    fn default() -> Self {
        Self {
            current: Route::default(),
            scroll_handles: std::array::from_fn(|_| ScrollHandle::new()),
        }
    }
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

    pub fn scroll_handle(&self, route: Route) -> ScrollHandle {
        self.scroll_handles[route.index()].clone()
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

    #[test]
    fn routes_keep_independent_scroll_positions() {
        let router = Router::default();
        let home = router.scroll_handle(Route::Home);
        home.set_offset(gpui::point(gpui::px(0.0), gpui::px(-240.0)));

        assert_eq!(
            router.scroll_handle(Route::Home).offset().y,
            gpui::px(-240.0)
        );
        assert_eq!(
            router.scroll_handle(Route::Options).offset().y,
            gpui::px(0.0)
        );
    }
}
