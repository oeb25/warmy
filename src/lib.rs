//! Hot-reloading, loadable and reloadable resources.
//!
//! # Foreword
//!
//! Resources are objects that live in a store and can be hot-reloaded – i.e. they can change
//! without you interacting with them. There are currently two types of resources supported:
//!
//!   - **Filesystem resources**, which are resources that live on the filesystem and have a real
//!     representation (i.e. a *file* for short).
//!   - **Logical resources**, which are resources that are computed and don’t directly require any
//!     I/O.
//!
//! Resources are referred to by *keys*. A *key* is a typed index that contains enough information
//! to uniquely identify a resource living in a store. You will find *filesystem keys* and *logical
//! keys*.
//!
//! This small introduction will give you enough information and examples to get your feet wet with
//! `warmy`. If you want to know more, feel free to visit the documentation of submodules.
//!
//! # Loading a resource
//!
//! *Loading* is the action of getting an object out of a given location. That location is often
//! your filesystem but it can also be a memory area – mapped files or memory parsing. In `warmy`,
//! loading is implemented *per-type*: this means you have to implement a trait on a type so that
//! any object of that type can be loaded. The trait to implement is [Load]. We’re interested in
//! four items:
//!
//!   - The [Store], which holds and caches resources.
//!   - The [Load::Key] associated type, used to tell `warmy` which kind of resource your type
//!     represents and what information the key must contain.
//!   - The [Load::Error] associated type, that is the error type used when loading fails.
//!   - The [Load::load] method, which is the method called to load your resource in a given store.
//!
//! ## `Store`
//!
//! A [Store] is responsible for holding and caching resources. Each [Store] is associated with a
//! *root*, which is a path on the filesystem all filesystem resources will come from. You create a
//! [Store] by giving it a [StoreOpt], which is used to customize the [Store] – if you don’t need
//! it, use `Store::default()`.
//!
//! ```
//! use warmy::{Store, StoreOpt};
//!
//! let res = Store::<()>::new(StoreOpt::default());
//!
//! match res {
//!   Err(e) => {
//!     eprintln!("unable to create the store: {:#?}", e);
//!   }
//!
//!   Ok(store) => ()
//! }
//! ```
//!
//! As you can see, the [Store] has a type variable. This type variable refers to the type of
//! *context* you want to use with your resource. For now we’ll use `()` as we don’t want contexts,
//! but more to come. Keep on reading.
//!
//! ## `Load::Key`
//!
//! This associated type must implement [Key], which is the class of types recognized as keys by
//! `warmy`. In theory, you shouldn’t worry about that trait because `warmy` already ships with some
//! key types.
//!
//! > If you really want to implement [Key], have a look at its documentation for further details.
//!
//! Keys are a core concept in `warmy` as they are objects that uniquely represent resources –
//! should they be on a filesystem or in memory. You will refer to your resources with those keys.
//!
//! Let’s dig in some key types.
//!
//! ### The classic: `FSKey`, the filesystem key
//!
//! [FSKey] is the type of key to choose if you want to refer to a resource on a filesystem. It’s
//! very easy to build one:
//!
//! ```
//! use warmy::FSKey;
//!
//! let my_key = FSKey::new("/foo/bar/zoo.json");
//! ```
//!
//! The paths you use in [FSKey] are always relative to the store’s root, which implements some kind
//! of a [VFS] for those keys.
//!
//! > Note: if you don’t use the leading `'/'`, the [FSKey] is still considered as if it was
//! > expressed with a leading `'/'`. Both `FSKey::new("/zulu.json")` and `FSKey::new("zulu.json")`
//! > refer to the exact same resource.
//!
//! ### Flexibility: `LogicalKey`, the memory key
//!
//! This type of key is a bit hard to wrap your finger around at first, because you might not need
//! it. This type of key enables you to create unique identifiers for resources that do not
//! *necessarily* exist on a filesystem. Those are like keys in a key-value store (think of the
//! *local storage* of your web browser, for instance).
//!
//! However, they come in **very handy** when coping with dependency graphs. More on that in a few
//! minutes – keep on reading!
//!
//! ```
//! use warmy::LogicalKey;
//!
//! let my_key = LogicalKey::new("586e6452-4bac-11e8-842f-0ed5f89f718b");
//! ```
//!
//! Logical keys are very simple to use and may contain any kind of information. However, for now,
//! they must be encoded with strings.
//!
//! ### Special case: dependency key
//!
//! A *dependency key* (a.k.a. [DepKey]) is a key used to express dependencies. Any type of key that
//! implements [Key] also implements `Into<DepKey>`, which comes in handy when you want to build
//! heterogenous lists of dependency keys.
//!
//! [DepKey] is either akin to a [FSKey] or [LogicalKey].
//!
//! ## `Load::Error`
//!
//! This associated type must be set to the type of error your loading implementation might
//! generate. For instance, if you load something with [serde-json], you might want to set it to
//! [serde_json::Error].
//!
//! > On a general note, you should always try to stick precise and accurate errors.Avoid simple
//! > types such as `String` or `u64` and prefer to use detailed, algebraic datatypes.
//!
//! ## `Load::load`
//!
//! This is the entry-point method. [Load::load] must be implemented in order for `warmy` to know
//! how to read the resource. Let’s implement it for two types: one that represents a resource on
//! the filesystem, one computed from memory.
//!
//! ```
//! use std::fs::File;
//! use std::io::{self, Read};
//! use std::path::PathBuf;
//! use warmy::{FSKey, Load, Loaded, LogicalKey, Storage};
//!
//! // The resource we want to take from a file.
//! struct FromFS(String);
//!
//! // The resource we want to compute from memory.
//! struct FromMem(usize);
//!
//! impl<C> Load<C> for FromFS {
//!   type Key = FSKey;
//!
//!   type Error = io::Error;
//!
//!   fn load(
//!     key: Self::Key,
//!     storage: &mut Storage<C>,
//!     _: &mut C
//!   ) -> Result<Loaded<Self>, Self::Error> {
//!     let mut fh = File::open(key.as_path())?;
//!     let mut s = String::new();
//!     fh.read_to_string(&mut s);
//!
//!     Ok(FromFS(s).into())
//!   }
//! }
//!
//! impl<C> Load<C> for FromMem {
//!   type Key = LogicalKey;
//!
//!   type Error = io::Error;
//!
//!   fn load(
//!     key: Self::Key,
//!     storage: &mut Storage<C>,
//!     _: &mut C
//!   ) -> Result<Loaded<Self>, Self::Error> {
//!     // this is a bit dummy, but why not?
//!     Ok(FromMem(key.as_str().len()).into())
//!   }
//! }
//! ```
//!
//! As you can see here, there’re a few new concepts:
//!
//!   - [Loaded]: A type you have to wrap your object in to express dependencies. Because it
//!     implements `From<T> for Loaded<T>`, you can use `.into()` to state you don’t have any
//!     dependencies.
//!   - [Storage]: This is the minimal structure that holds and caches your resources. A [Store] is
//!     actually the *interface structure* you will handle in your client code.
//!
//! ## Express your dependencies with `Loaded`
//!
//! An object of type [Loaded] gives information to `warmy` about your dependencies. Upon loading –
//! i.e. your resource is successfully *loaded* – you can tell `warmy` which resources your loaded
//! resource depends on. This is a bit tricky, though, because a diffference is important to make
//! there.
//!
//! When you implement [Load::load], you are handed a [Storage]. You can use that [Storage] to load
//! additional resources and gather them in your resources. When those additional resources get
//! reloaded, if you directly embed the resources in your object, you will automatically see the
//! automated resources. However, if you don’t express a *dependency relationship* to those
//! resources, your former resource will not reload – it will just use automatically-synced
//! resources, but it will not reload itself. This is a bit touchy but let’s take an example of a
//! typical situation where you might want to use dependencies and then dependencies graphs:
//!
//!   1. You want to load an object that is represented by aggregation of several values /
//!      resources.
//!   2. You choose to use a *logical resource* and guess all the files to load from a [LogicalKey].
//!   3. When you implement [Load::load], you open several files, load them into memory, compose
//!      them and finally end up with your object.
//!   4. You return your object from [Load::load] with no dependencies (i.e. you use `.into()` on
//!      it).
//!
//! What is going to happen here is that if any of the files your resource depends on changes,
//! since they don’t have a proper resource in the store, your object will see nothing. A typical
//! solution there is to load those files as proper resources (by using [FSKey]) and put those
//! keys in the returned [Loaded] object to express that you *depend on the reloading of the objects
//! referred by these keys*. It’s a bit touchy but you will eventually find yourself in a situation
//! when this [Loaded] thing will help you. You will then use `Loaded::with_deps`. See the
//! documentation of [Loaded] for further information.
//!
//! > Fun fact: [LogicalKey] was introduced to solve that problem along with dependency graphs.
//!
//! ## Let’s get some things!
//!
//! When you have implemented [Load], you’re set and ready to get (cached) resources. You have
//! several functions to achieve that goal:
//!
//!   - [Store::get], used to get a resource. This will effectively load it if it’s the first time
//!     it’s asked. If it’s not, it will use a cached version.
//!   - [Store::get_proxied], a special version of [Store::get]. If the initial loading (non-cached)
//!     fails to load (missing resource, fail to parse, whatever), a *proxy* will be used – passed
//!     in to [Store::get_proxied]. This value is lazy though, so if the loading succeeds, that
//!     value won’t ever be evaluated.
//!
//! Let’s focus on [Store::get] for this tutorial.
//!
//! ```
//! use std::fs::File;
//! use std::io::{self, Read};
//! use std::path::PathBuf;
//! use warmy::{FSKey, Load, Loaded, LogicalKey, Res, Store, StoreOpt, Storage};
//!
//! // The resource we want to take from a file.
//! struct FromFS(String);
//!
//! impl<C> Load<C> for FromFS {
//!   type Key = FSKey;
//!
//!   type Error = io::Error;
//!
//!   fn load(
//!     key: Self::Key,
//!     storage: &mut Storage<C>,
//!     _: &mut C
//!   ) -> Result<Loaded<Self>, Self::Error> {
//!     let mut fh = File::open(key.as_path())?;
//!     let mut s = String::new();
//!     fh.read_to_string(&mut s);
//!
//!     Ok(FromFS(s).into())
//!   }
//! }
//!
//! fn main() {
//!   // we don’t need a context, so we’re using this mutable reference to unit
//!   let ctx = &mut ();
//!   let mut store: Store<()> = Store::new(StoreOpt::default()).expect("store creation");
//!
//!   let my_resource = store.get::<_, FromFS>(&FSKey::new("/foo/bar/zoo.json"), ctx);
//!
//!   // …
//!
//!   // imagine that you’re in an event loop now and the resource has changed
//!   store.sync(ctx); // synchronize all resources (e.g. my_resource) with the filesystem
//! }
//! ```
//!
//! # Reloading a resource
//!
//! Most of the interesting concept of `warmy` is to enable you to hot-reload resources without
//! having to re-run your application. This is done via two items:
//!
//!   - [Load::reload], a method called whenever an object must be reloaded.
//!   - [Store::sync], a method to synchronize a [Store] with the part of the filesystem it’s
//!     responsible for.
//!
//! The [Load::reload] function is very straight-forward: it’s called when the resource changes.
//! This situation happens:
//!
//!   - Either when the resource is on the filesystem (the file changes).
//!   - Or if it’s a dependent resource of one that has reloaded.
//!
//! See the documentation of [Load::reload] for further details.
//!
//! # Context
//!
//! A context is a special value you can access to via a mutable references when loading or
//! reloading. If you don’t need any, it’s highly recommended not to use `()` when implementing
//! `Load<C>` but leave it as polymorphic value so that it composes better – i.e. `impl<C> Load<C>`.
//!
//! If you’re writing a library and need to have access to a specific value in a context, it’s also
//! recommended not to set the context type variable to the type of your context directly. If you do
//! that, no one will be able to use your library because types won’t match. A typical way to deal
//! with that is by constraining a polymorphic type variable. For instance:
//!
//! ```
//! use std::io;
//! use warmy::{Load, Loaded, LogicalKey, Storage};
//!
//! struct Foo;
//!
//! struct Ctx {
//!   nb_res_loaded: usize
//! }
//!
//! trait HasCtx {
//!   fn get_ctx(&mut self) -> &mut Ctx;
//! }
//!
//! impl HasCtx for Ctx {
//!   fn get_ctx(&mut self) -> &mut Ctx {
//!     self
//!   }
//! }
//!
//! impl<C> Load<C> for Foo where C: HasCtx {
//!   type Key = LogicalKey;
//!
//!   type Error = io::Error; // should be the never type, !, but not stable yet
//!
//!   fn load(
//!     key: Self::Key,
//!     storage: &mut Storage<C>,
//!     ctx: &mut C
//!   ) -> Result<Loaded<Self>, Self::Error> {
//!     ctx.get_ctx().nb_res_loaded += 1;
//!
//!     Ok(Foo.into())
//!   }
//! }
//! ```
//!
//! # Load methods
//!
//! `warmy` supports load methods. Those are used to specify several ways to load an object of a
//! given type. By default, [Load] is implemented with the *default method* – which is `()`. If you
//! want more methods, you can set the type parameter to something else when implementing [Load].
//!
//! You can also find several [methods] centralized in here, but you definitely don’t have to use
//! them. In theory, those will be removed and placed into other crates to add automatic
//! implementations.
//!
//!
//! [Load]: load/trait.Load.html
//! [Loaded]: load/struct.Loaded.html
//! [Load::Key]: load/trait.Load.html#associatedtype.Key
//! [Load::Error]: load/trait.Load.html#associatedtype.Error
//! [Load::load]: load/trait.Load.html#tymethod.load
//! [Load::reload]: load/trait.Load.html#tymethod.reload
//! [Key]: key/trait.Key.html
//! [FSKey]: key/struct.FSKey.html
//! [LogicalKey]: key/struct.LogicalKey.html
//! [DepKey]: key/struct.DepKey.html
//! [Store]: load/struct.Store.html
//! [Store::get]: load/struct.Store.html#method.get
//! [Store::get_proxied]: load/struct.Store.html#method.get_proxied
//! [Store::sync]: load/struct.Store.html#method.sync
//! [StoreOpt]: load/struct.StoreOpt.html
//! [Storage]: load/struct.Storage.html
//! [serde-json]: https://crates.io/crates/serde_json
//! [serde_json::Error]: https://docs.serde.rs/serde_json/struct.Error.html
//! [methods]: methods/index.html
//! [VFS]: https://en.wikipedia.org/wiki/Virtual_file_system

extern crate any_cache;
extern crate notify;

pub mod key;
pub mod load;
pub mod methods;
pub mod res;

pub use key::{DepKey, FSKey, Key, LogicalKey};
pub use load::{Load, Loaded, Storage, Store, StoreError, StoreErrorOr, StoreOpt};
pub use res::Res;
