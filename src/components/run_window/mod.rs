mod hash_store_tab;
mod key_store_tab;
mod run_tab;

use leptos::{component, view, IntoView};

use self::hash_store_tab::HashStoreTab;
use self::key_store_tab::KeyStoreTab;
use self::run_tab::RuntimeTab;
use crate::components::navbar::{Navbar, Tab};

pub use self::hash_store_tab::HashedData;
pub use self::key_store_tab::{SignedData, SigningKeys};

#[component]
pub fn RunWindow() -> impl IntoView {
    view! {
        <Navbar default_tab="Runtime">
            <Tab name="Runtime">
                <RuntimeTab />
            </Tab>
            <Tab name="Key Store">
                <KeyStoreTab />
            </Tab>
            <Tab name="Hash Store">
                <HashStoreTab />
            </Tab>
        </Navbar>
    }
}
