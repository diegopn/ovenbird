#![cfg_attr(docsrs, feature(doc_cfg))]
//! # Rust Libsecret bindings
//!
//! This library contains safe Rust bindings for
//! [Libsecret](https://gitlab.gnome.org/GNOME/libsecret/), a library that
//! offers access to the Secret Service API.
//!
//! See also
//!
//! - [gtk-rs project overview](https://gtk-rs.org/)
//!
//! ## Usage
//!
//! You can add libsecret by adding it in your `Cargo.toml` file:
//!
//! ```toml
//! [dependencies.secret]
//! package = "libsecret"
//! version = "0.x.y"
//! ```
//!
//! ### Sync versus async API
//!
//! The following examples use synchronous functions for simplicit.  In a GUI
//! application however you should use the async Future-based API (e.g
//! [`password_store_future`] and [`password_lookup_future`]) instead to avoid
//! blocking the UI.
//!
//! ### Define a password schema
//!
//! Each stored password has a set of attributes which are later used to lookup
//! the password. The names and types of the attributes are defined in a schema.
//! The schema is usually defined once globally. Here’s how to define a schema:
//!
//! ```no_run
//! # use std::collections::HashMap;
//! use libsecret::*;
//! let mut attributes = HashMap::new();
//! attributes.insert("number", SchemaAttributeType::Integer);
//! attributes.insert("string", SchemaAttributeType::String);
//! attributes.insert("even", SchemaAttributeType::Boolean);
//!
//! let schema = Schema::new("com.example.app", SchemaFlags::NONE, attributes);
//! ```
//!
//! ### Store a password
//!
//! Each stored password has a set of attributes which are later used to lookup
//! the password. The attributes should not contain secrets, as they are not
//! stored in an encrypted fashion.
//!
//! ```no_run
//! # use std::collections::HashMap;
//! # let schema = libsecret::Schema::new("com.example.app", libsecret::SchemaFlags::NONE, HashMap::new());
//! let mut attributes = std::collections::HashMap::new();
//! attributes.insert("number", "8");
//! attributes.insert("string", "eight");
//! attributes.insert("even", "true");
//!
//! let collection = libsecret::COLLECTION_DEFAULT;
//! libsecret::password_store_sync(
//!     Some(&schema),
//!     attributes,
//!     Some(&collection),
//!     "The Label",
//!     "the password",
//!     gio::Cancellable::NONE
//! )?;
//! # Ok::<(), glib::Error>(())
//! ```
//!
//! ### Lookup a password
//!
//! Each stored password has a set of attributes which are used to lookup the
//! password. If multiple passwords match the lookup attributes, then the one
//! stored most recently is returned.
//!
//! This first example looks up a password asynchronously, and is appropriate
//! for GUI applications so that the UI does not block.
//!
//! ```no_run
//! # use std::collections::HashMap;
//! # let schema = libsecret::Schema::new("com.example.app", libsecret::SchemaFlags::NONE, HashMap::new());
//! let mut attributes = std::collections::HashMap::new();
//! attributes.insert("number", "8");
//! attributes.insert("even", "true");
//!
//! let password: Option<glib::GString> = libsecret::password_lookup_sync(Some(&schema), attributes, gio::Cancellable::NONE)?;
//! # Ok::<(), glib::Error>(())
//! ```
//! ### Remove a password
//!
//! Each stored password has a set of attributes which are used to find which
//! password to remove. If multiple passwords match the attributes, then the one
//! stored most recently is removed.
//!
//! This first example removes a password asynchronously, and is appropriate for
//! GUI applications so that the UI does not block.
//!
//! ```no_run
//! # use std::collections::HashMap;
//! # let schema = libsecret::Schema::new("com.example.app", libsecret::SchemaFlags::NONE, HashMap::new());
//! let mut attributes = HashMap::new();
//! attributes.insert("number", "8");
//! attributes.insert("even", "true");
//!
//! libsecret::password_clear_sync(Some(&schema), attributes, gio::Cancellable::NONE)?;
//! # Ok::<(), glib::Error>(())
//! ```

#[allow(unused_imports, clippy::all)]
mod auto;

pub use auto::functions::*;
pub use auto::*;
pub use functions::*;

mod collection;
mod enums;
mod functions;
mod hashtable;
#[allow(clippy::too_many_arguments)]
mod item;
mod prompt;
#[cfg(any(feature = "v0_19", docsrs))]
#[cfg_attr(docsrs, doc(cfg(feature = "v0_19")))]
mod retrievable;
mod schema;
mod schema_attribute;
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
mod service;
mod value;

pub use value::Value;

pub use ffi;

pub mod prelude {
    pub use super::auto::traits::*;
    pub use super::collection::CollectionExtManual;
    pub use super::prompt::PromptExtManual;
    #[cfg(any(feature = "v0_19", docsrs))]
    #[cfg_attr(docsrs, doc(cfg(feature = "v0_19")))]
    pub use super::retrievable::RetrievableExtManual;
    pub use super::service::ServiceExtManual;
}
