//! FEEL context.

use crate::diagnostics;
use crate::errors::*;
use crate::message::{MessageArg, Size};
use crate::names::Name;
use crate::qualified_names::QualifiedName;
use crate::strings::{ToFeelString, write_separated};
use crate::value_null;
use crate::values::Value;
use dsntk_common::{DsntkError, Jsonify, Result};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Bound;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

/// The order in which the entries of a context were written, which is the order they are shown
/// in. The entries are kept in the order of their names, which finds an entry and decides what
/// two contexts have in common; this says where that is not the order of writing.
#[derive(Clone, Default)]
enum Written {
  /// In the order of their names.
  #[default]
  ByName,
  /// The names in the order of writing, each once. An entry that is added is added at its
  /// end, in constant time; an entry that is replaced keeps its place.
  Names(Vec<Name>),
  /// For a record: the places of its names in the order of writing, one list for all the
  /// records of its shape.
  Places(Arc<[u32]>),
}

/// The entries of a context: kept in the order of their names, each name once, with the order
/// they were written in.
#[derive(Clone, Default)]
struct FeelContextEntries {
  store: Store,
  written: Written,
}

/// The entries of a context, in the order of their names, each name once.
#[derive(Clone)]
enum Store {
  /// A map from the names to the values.
  Map(BTreeMap<Name, Value>),
  /// The names, ascending and each once, and the value of each: the form of a record that has
  /// its names in common with other records ([FeelContext::from_shared_names]). It becomes a
  /// map when an entry changes.
  Record(Arc<[Name]>, Vec<Value>),
}

impl Default for Store {
  fn default() -> Self {
    Self::Map(BTreeMap::new())
  }
}

impl From<Store> for FeelContextEntries {
  /// The entries of a store, written in the order of their names.
  fn from(store: Store) -> Self {
    Self { store, written: Written::ByName }
  }
}

/// The entries of a context in the order of their names.
enum Entries<'a> {
  Map(std::collections::btree_map::Iter<'a, Name, Value>),
  Record(std::iter::Zip<std::slice::Iter<'a, Name>, std::slice::Iter<'a, Value>>),
}

impl<'a> Iterator for Entries<'a> {
  type Item = (&'a Name, &'a Value);

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      Self::Map(entries) => entries.next(),
      Self::Record(entries) => entries.next(),
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    match self {
      Self::Map(entries) => entries.size_hint(),
      Self::Record(entries) => entries.size_hint(),
    }
  }
}

impl ExactSizeIterator for Entries<'_> {}

/// The names of a context from one name on, in their order.
enum NamesFrom<'a> {
  Map(std::collections::btree_map::Range<'a, Name, Value>),
  Record(std::slice::Iter<'a, Name>),
}

impl<'a> Iterator for NamesFrom<'a> {
  type Item = &'a Name;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      Self::Map(names) => names.next().map(|(name, _)| name),
      Self::Record(names) => names.next(),
    }
  }
}

/// The entries of a context in the order they were written in.
enum WrittenEntries<'a> {
  ByName(Entries<'a>),
  Names(std::slice::Iter<'a, Name>, &'a BTreeMap<Name, Value>),
  Places(std::slice::Iter<'a, u32>, &'a [Name], &'a [Value]),
}

impl<'a> Iterator for WrittenEntries<'a> {
  type Item = (&'a Name, &'a Value);

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      Self::ByName(entries) => entries.next(),
      // every name of the list is a name of the map
      Self::Names(names, map) => names.find_map(|name| map.get_key_value(name)),
      Self::Places(places, names, values) => places.find_map(|place| names.get(*place as usize).zip(values.get(*place as usize))),
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    match self {
      Self::ByName(entries) => entries.size_hint(),
      Self::Names(names, _) => names.size_hint(),
      Self::Places(places, ..) => places.size_hint(),
    }
  }
}

impl ExactSizeIterator for WrittenEntries<'_> {}

impl FeelContextEntries {
  /// The entries in the order they were written in.
  fn iter_written(&self) -> WrittenEntries<'_> {
    match (&self.store, &self.written) {
      (Store::Map(map), Written::Names(names)) => WrittenEntries::Names(names.iter(), map),
      (Store::Record(names, values), Written::Places(places)) => WrittenEntries::Places(places.iter(), names, values),
      (store, _) => WrittenEntries::ByName(store.iter()),
    }
  }

  /// Adds an entry, or replaces the entry of its name, which keeps its place in the order of
  /// writing; a new entry is the last one. Returns the value it replaces.
  fn insert(&mut self, name: Name, value: Value) -> Option<Value> {
    self.map_mut();
    let Store::Map(map) = &mut self.store else {
      unreachable!("a record became a map");
    };
    match &mut self.written {
      Written::ByName => {
        // after the last name, the order of the names is still the order of writing
        if map.last_key_value().is_some_and(|(last, _)| *last >= name) && !map.contains_key(&name) {
          let mut names: Vec<Name> = map.keys().cloned().collect();
          names.push(name.clone());
          self.written = Written::Names(names);
        }
      }
      Written::Names(names) => {
        if !map.contains_key(&name) {
          names.push(name.clone());
        }
      }
      // a map has no places
      Written::Places(_) => self.written = Written::ByName,
    }
    map.insert(name, value)
  }

  /// Removes the entry of a name and returns its value.
  fn remove(&mut self, name: &Name) -> Option<Value> {
    let removed = self.map_mut().remove(name)?;
    if let Written::Names(names) = &mut self.written {
      // the entry written last is the one that is mostly removed
      if names.last() == Some(name) {
        names.pop();
      } else if let Some(place) = names.iter().position(|written| written == name) {
        names.remove(place);
      }
    }
    Some(removed)
  }

  /// The value of the entry of a name, to be changed.
  fn get_mut(&mut self, name: &Name) -> Option<&mut Value> {
    self.map_mut().get_mut(name)
  }

  /// The entries as a map, to be changed: a record becomes one, and the places of its names
  /// become the names in the order of writing. The map is changed only through
  /// [FeelContextEntries::insert] and [FeelContextEntries::remove], which keep that order.
  fn map_mut(&mut self) -> &mut BTreeMap<Name, Value> {
    if let (Store::Record(names, _), Written::Places(places)) = (&self.store, &self.written) {
      let names = places.iter().filter_map(|place| names.get(*place as usize).cloned()).collect();
      self.written = Written::Names(names);
    }
    self.store.map_mut()
  }

  fn iter(&self) -> Entries<'_> {
    self.store.iter()
  }

  fn values(&self) -> impl Iterator<Item = &Value> {
    self.store.values()
  }

  fn len(&self) -> usize {
    self.store.len()
  }

  fn is_empty(&self) -> bool {
    self.store.is_empty()
  }

  fn get<Q: Ord + ?Sized>(&self, name: &Q) -> Option<&Value>
  where
    Name: Borrow<Q>,
  {
    self.store.get(name)
  }

  fn contains_key<Q: Ord + ?Sized>(&self, name: &Q) -> bool
  where
    Name: Borrow<Q>,
  {
    self.store.contains_key(name)
  }

  fn names_from(&self, name: &str) -> NamesFrom<'_> {
    self.store.names_from(name)
  }
}

impl Store {
  fn iter(&self) -> Entries<'_> {
    match self {
      Self::Map(map) => Entries::Map(map.iter()),
      Self::Record(names, values) => Entries::Record(names.iter().zip(values)),
    }
  }

  fn values(&self) -> impl Iterator<Item = &Value> {
    self.iter().map(|(_, value)| value)
  }

  fn len(&self) -> usize {
    match self {
      Self::Map(map) => map.len(),
      Self::Record(_, values) => values.len(),
    }
  }

  fn is_empty(&self) -> bool {
    self.len() == 0
  }

  fn get<Q: Ord + ?Sized>(&self, name: &Q) -> Option<&Value>
  where
    Name: Borrow<Q>,
  {
    match self {
      Self::Map(map) => map.get(name),
      Self::Record(names, values) => names
        .binary_search_by(|entry| Borrow::<Q>::borrow(entry).cmp(name))
        .ok()
        .and_then(|index| values.get(index)),
    }
  }

  fn contains_key<Q: Ord + ?Sized>(&self, name: &Q) -> bool
  where
    Name: Borrow<Q>,
  {
    self.get(name).is_some()
  }

  /// The names from the first one not less than `name` on, in their order.
  fn names_from(&self, name: &str) -> NamesFrom<'_> {
    match self {
      Self::Map(map) => NamesFrom::Map(map.range::<str, _>((Bound::Included(name), Bound::Unbounded))),
      Self::Record(names, _) => NamesFrom::Record(names[names.partition_point(|entry| Borrow::<str>::borrow(entry) < name)..].iter()),
    }
  }

  /// The entries as a map, to be changed: a record becomes one.
  fn map_mut(&mut self) -> &mut BTreeMap<Name, Value> {
    if let Self::Record(names, values) = self {
      // ascending and each name once, so the map is built in one pass
      *self = Self::Map(names.iter().cloned().zip(std::mem::take(values)).collect());
    }
    match self {
      Self::Map(map) => map,
      Self::Record(..) => unreachable!("a record became a map"),
    }
  }
}

impl PartialEq for FeelContextEntries {
  /// The same entries, in whatever order they were written.
  fn eq(&self, other: &Self) -> bool {
    self.len() == other.len() && self.iter().eq(other.iter())
  }
}

impl fmt::Debug for FeelContextEntries {
  /// The form of a map, whatever the form of the entries.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_map().entries(self.iter()).finish()
  }
}

/// What an entry of a context is looked up by: a [Name] or its text.
pub trait KeyText {
  /// The text of the name.
  fn key_text(&self) -> &str;
}

impl KeyText for Name {
  fn key_text(&self) -> &str {
    self.as_str()
  }
}

impl KeyText for str {
  fn key_text(&self) -> &str {
    self
  }
}

/// The FEEL context.
///
/// A context remembers its nesting depth once it was read, until its entries change: every
/// method that changes an entry goes through `FeelContext::entries_mut`, which forgets it, so a
/// change costs one store and a context that is only written (a scope frame) never computes it.
/// Reading it again after a change looks at each entry once, and an entry that is a list or a
/// context read since it last changed answers in constant time.
///
/// The remembered depth is no cache: it is derived from the entries alone and exact whenever it is
/// set. It goes from "not known" to the depth on a read and back on a change, a clone copies it
/// with the identical entries, and no other state is kept. Two reads of one context, through any
/// handles and on any threads, store the same number, so a context read through a shared handle
/// stays equal to one never read.
///
/// Equality is that of the entries; the debug form is `FeelContext({..})` with the entries.
pub struct FeelContext {
  /// The entries; not set yet in a context whose entries are read on the first use
  /// ([FeelContext::deferred]).
  built: OnceLock<FeelContextEntries>,
  /// What reads the entries on the first use, in a context that was made without them.
  source: Option<Arc<dyn DeferredEntries>>,
  /// The nesting depth of this context when it was read since the last change, 0 otherwise
  /// (a context is at least one level deep).
  depth: AtomicU32,
  /// Whether a key of this context is written in another form than FEEL reads a name
  /// ([crate::names::is_in_name_form]): then an entry that is not found by the text of a name
  /// is searched by the tokens of its key. Said by whoever builds the context from keys that
  /// are not FEEL's own ([FeelContext::note_key_forms]); an entry set later does not change it.
  other_forms: AtomicBool,
}

/// The entries of a context that are read when the context is first used.
pub trait DeferredEntries: Send + Sync {
  /// Reads the context: always the same one.
  fn build(&self) -> FeelContext;
}

impl Default for FeelContext {
  fn default() -> Self {
    Self::of(FeelContextEntries::default())
  }
}

impl Clone for FeelContext {
  fn clone(&self) -> Self {
    // a context that was not used yet stays one: its clone is read from the same source
    let built = OnceLock::new();
    if let Some(entries) = self.built.get() {
      let _ = built.set(entries.clone());
    }
    Self {
      built,
      source: self.source.clone(),
      depth: AtomicU32::new(self.depth.load(Ordering::Relaxed)),
      other_forms: AtomicBool::new(self.other_forms.load(Ordering::Relaxed)),
    }
  }
}

impl PartialEq for FeelContext {
  fn eq(&self, other: &Self) -> bool {
    self.entries() == other.entries()
  }
}

impl fmt::Debug for FeelContext {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("FeelContext").field(self.entries()).finish()
  }
}

impl FromIterator<(Name, Value)> for FeelContext {
  /// Builds a context from entries, with the entries that [FeelContext::insert] gives for one
  /// after another: an entry with the name of an earlier one replaces it. The entries that come
  /// in ascending order of their names, without a repeated name, are built in one pass, without
  /// a search per entry; from the first entry out of this order on, each is inserted.
  fn from_iter<T: IntoIterator<Item = (Name, Value)>>(iter: T) -> Self {
    // the entries of a vector are taken as they are, without a copy
    let mut ascending: Vec<(Name, Value)> = iter.into_iter().collect();
    let in_order = ascending.windows(2).position(|pair| pair[0].0 >= pair[1].0).map_or(ascending.len(), |index| index + 1);
    let rest = ascending.split_off(in_order);
    // sorted and without equal names, so the result does not depend on how the map treats either
    let mut entries = FeelContextEntries::from(Store::Map(BTreeMap::from_iter(ascending)));
    // the others in the order they come in, which is the order they were written in
    for (name, value) in rest {
      entries.insert(name, value);
    }
    Self::of(entries)
  }
}

impl FeelContext {
  /// Builds a context from entries that come in ascending order of their names, each name
  /// once, in one pass and without the check of [FromIterator]. Entries in another order give a
  /// context too, but which of two entries with one name it keeps is not defined.
  pub fn from_ascending_entries(entries: impl IntoIterator<Item = (Name, Value)>) -> Self {
    Self::of(FeelContextEntries::from(Store::Map(BTreeMap::from_iter(entries))))
  }

  /// [FeelContext::from_ascending_entries] for entries that were written in another order:
  /// `places[i]` is the place among `entries` of the entry written at `i`. The order of
  /// writing is taken when `places` has every place once and no two entries have one name: the
  /// names at the places are then the names of the context, each once. That is checked on the
  /// places and the number of entries, without a comparison of names; other places leave the
  /// order of the names.
  pub fn from_ascending_entries_written(entries: Vec<(Name, Value)>, places: &[u32]) -> Self {
    let count = entries.len();
    let mut seen = vec![false; count];
    let each_once = places.len() == count && places.iter().all(|place| seen.get_mut(*place as usize).is_some_and(|seen| !std::mem::replace(seen, true)));
    let by_name = places.iter().enumerate().all(|(index, place)| *place as usize == index);
    let names: Vec<Name> = if each_once && !by_name {
      places.iter().filter_map(|place| entries.get(*place as usize).map(|entry| entry.0.clone())).collect()
    } else {
      vec![]
    };
    let mut built = FeelContextEntries::from(Store::Map(BTreeMap::from_iter(entries)));
    if names.len() == count && built.len() == count {
      built.written = Written::Names(names);
    }
    Self::of(built)
  }

  /// Builds a context of the entries `names[i]: values[i]`. The names are shared: the records
  /// of a list that have the same names hold one list of them, and each its values. The names
  /// must be ascending, each once, and as many as the values; a context of other names and
  /// values is not defined (it stays memory-safe: a lookup may miss an entry).
  pub fn from_shared_names(names: Arc<[Name]>, values: Vec<Value>) -> Self {
    debug_assert!(names.len() == values.len() && names.windows(2).all(|pair| pair[0] < pair[1]));
    Self::of(FeelContextEntries::from(Store::Record(names, values)))
  }

  /// Says the order in which the entries of this context were written: `names` has every name
  /// of it once. A list that is not that leaves the order as it is. The names in their own
  /// order need no list.
  pub fn with_written_names(mut self, names: Vec<Name>) -> Self {
    self.set_written_names(names);
    self
  }

  /// [FeelContext::with_written_names] for a context that is held.
  pub fn set_written_names(&mut self, names: Vec<Name>) {
    let entries = self.entries_mut();
    let mut distinct: Vec<&Name> = names.iter().collect();
    distinct.sort_unstable();
    distinct.dedup();
    if names.len() == entries.len() && distinct.len() == names.len() && names.iter().all(|name| entries.contains_key(name)) {
      entries.map_mut();
      let by_name = names.windows(2).all(|pair| pair[0] < pair[1]);
      entries.written = if by_name { Written::ByName } else { Written::Names(names) };
    }
  }

  /// Says the order in which the entries of a context of shared names ([FeelContext::from_shared_names])
  /// were written: `places[i]` is the place among the names of the entry written at `i`, and
  /// every place comes once. One list serves all the records of a shape. A list that is not
  /// that leaves the order as it is.
  pub fn with_written_places(mut self, places: Arc<[u32]>) -> Self {
    let entries = self.entries();
    let fits = matches!(entries.store, Store::Record(..)) && places.len() == entries.len();
    if fits && let Some(entries) = self.built.get_mut() {
      entries.written = Written::Places(places);
    }
    self
  }

  /// A context of `entries`.
  fn of(entries: FeelContextEntries) -> Self {
    Self {
      built: OnceLock::from(entries),
      source: None,
      depth: AtomicU32::new(0),
      other_forms: AtomicBool::new(false),
    }
  }

  /// A context whose entries `source` reads when the context is first used; `depth` is its
  /// nesting depth, which is known without the entries.
  pub fn deferred(source: Arc<dyn DeferredEntries>, depth: u32) -> Self {
    Self {
      built: OnceLock::new(),
      source: Some(source),
      depth: AtomicU32::new(depth),
      other_forms: AtomicBool::new(false),
    }
  }

  /// Returns `true` when the entries of this context are read: always, but in a context made
  /// by [FeelContext::deferred] that was not used yet.
  pub fn is_built(&self) -> bool {
    self.built.get().is_some()
  }

  /// The entries, read now when they were not yet.
  #[inline]
  fn entries(&self) -> &FeelContextEntries {
    match self.built.get() {
      Some(entries) => entries,
      None => self.build(),
    }
  }

  #[cold]
  fn build(&self) -> &FeelContextEntries {
    self.built.get_or_init(|| match &self.source {
      Some(source) => {
        let mut context = source.build();
        context.entries();
        if context.has_other_key_forms() {
          self.other_forms.store(true, Ordering::Relaxed);
        }
        context.built.take().unwrap_or_default()
      }
      None => FeelContextEntries::default(),
    })
  }
}

impl TryFrom<Value> for FeelContext {
  type Error = DsntkError;
  /// Converts [Value] to [FeelContext].
  /// The context is moved out of a value that holds it alone, and copied out of a shared one
  /// (a shallow copy: each entry clone shares what the entry holds).
  fn try_from(value: Value) -> Result<Self, Self::Error> {
    let Value::Context(ctx) = value else {
      return Err(err_value_is_not_a_context(&value));
    };
    Ok(Arc::unwrap_or_clone(ctx))
  }
}

impl From<FeelContext> for Value {
  /// Converts [FeelContext] to [Value], which holds it alone.
  fn from(ctx: FeelContext) -> Self {
    Value::Context(Arc::new(ctx))
  }
}

impl MessageArg for FeelContext {
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(self, f)
  }

  fn size(&self) -> Option<Size> {
    Some(Size::Entries(self.entries().len()))
  }
}

impl fmt::Display for FeelContext {
  /// Convert [FeelContext] to string.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("{")?;
    write_separated(f, self.entries().iter_written(), |f, (name, value)| {
      if name.is_empty() { write!(f, r#""": {value}"#) } else { write!(f, "{name}: {value}") }
    })?;
    f.write_str("}")
  }
}

impl ToFeelString for FeelContext {
  /// Converts [FeelContext] to FEEL string.
  fn to_feel_string(&self) -> String {
    format!(
      "{{{}}}",
      self
        .entries()
        .iter_written()
        .map(|(name, value)| {
          let name_str = format!("{name}");
          let padded_name_str = match name_str.as_str() {
            "{" | "}" | ":" | "," => format!("\"{name_str}\""),
            "\"" => "\"\\\"\"".to_string(),
            _ => name_str,
          };
          format!("{padded_name_str}: {}", value.to_feel_string())
        })
        .collect::<Vec<String>>()
        .join(", ")
    )
  }
}

impl Jsonify for FeelContext {
  /// Converts [FeelContext] to JSON string.
  fn jsonify(&self) -> String {
    format!(
      "{{{}}}",
      self
        .entries()
        .iter_written()
        .map(|(name, value)| format!("{}: {}", crate::values::json_string(&name.to_string()), value.jsonify()))
        .collect::<Vec<String>>()
        .join(", ")
    )
  }
}

impl FeelContext {
  /// Creates a new, empty context.
  pub fn new() -> Self {
    Self::default()
  }

  /// Says that the keys of this context come from outside FEEL: when one of them is written in
  /// another form than FEEL reads a name (`Date - Time` for the name `Date-Time`), an entry
  /// that is not found by the text of a name is searched by the tokens of its key from now on.
  /// The keys themselves stay as they are.
  pub fn note_key_forms(&mut self) {
    let other = self.entries().iter().any(|(name, _)| !crate::names::is_in_name_form(name.as_str()));
    self.other_forms.store(other, Ordering::Relaxed);
  }

  /// Says what [FeelContext::note_key_forms] finds, for a context whose keys were looked at
  /// already (the records of a list that share their names).
  pub fn with_other_key_forms(self, other: bool) -> Self {
    self.other_forms.store(other, Ordering::Relaxed);
    self
  }

  /// Returns `true` when a key of this context is known to be written in another form than
  /// FEEL reads a name ([FeelContext::note_key_forms]).
  pub fn has_other_key_forms(&self) -> bool {
    self.other_forms.load(Ordering::Relaxed)
  }

  /// The value of the entry that `name` reads: the entry of this text, or in a context with
  /// keys in other forms the first entry whose key is this name.
  #[inline]
  fn find<Q: Ord + KeyText + ?Sized>(&self, name: &Q) -> Option<&Value>
  where
    Name: Borrow<Q>,
  {
    let entries = self.entries();
    match entries.get(name) {
      None if self.other_forms.load(Ordering::Relaxed) => entries.iter().find(|(key, _)| crate::names::is_name(key.as_str(), name.key_text())).map(|(_, value)| value),
      found => found,
    }
  }

  /// Returns `true` if context contains an entry specified by **name** (a [Name] or its text).
  pub fn contains_entry<Q: Ord + KeyText + ?Sized>(&self, name: &Q) -> bool
  where
    Name: Borrow<Q>,
  {
    self.find(name).is_some()
  }

  /// Returns `true` if this [FeelContext] contains an entry specified by [QualifiedName].
  pub fn contains_entries(&self, qname: &QualifiedName) -> bool {
    self.contains_deep(qname.as_slice())
  }

  /// Returns a value of an entry with specified name.
  pub fn get_entry(&self, name: &Name) -> Option<&Value> {
    self.find(name)
  }

  /// Sets a value for specified entry name.
  pub fn set_entry(&mut self, name: &Name, value: Value) {
    self.put(name.clone(), value);
  }

  /// Inserts an entry, taking the name (an entry with the same name is replaced, like [FeelContext::set_entry]).
  pub fn insert(&mut self, name: Name, value: Value) {
    self.put(name, value);
  }

  /// Sets a value for specified entry name like [FeelContext::set_entry], but clones the name only
  /// when the entry is new: for entries that are written again and again (iteration variables).
  /// For a new entry it costs one more lookup than [FeelContext::set_entry].
  ///
  /// Returns the value it replaces, or `None` when the entry is new: so a value moved into an entry
  /// is taken back out by assigning a placeholder.
  pub fn assign_entry(&mut self, name: &Name, value: Value) -> Option<Value> {
    let entries = self.entries_mut();
    if let Some(entry) = entries.get_mut(name) {
      Some(std::mem::replace(entry, value))
    } else {
      entries.insert(name.clone(), value);
      None
    }
  }

  /// Removes a value of an entry with specified name.
  pub fn remove_entry(&mut self, name: &Name) -> Option<Value> {
    self.entries_mut().remove(name)
  }

  /// Sets a null value for specified entry.
  pub fn set_null(&mut self, name: Name) {
    self.put(name, value_null!());
  }

  /// Returns a list of all [FeelContext] entries, in the order they were written in.
  pub fn get_entries(&self) -> Vec<(&Name, &Value)> {
    self.entries().iter_written().collect::<Vec<(&Name, &Value)>>()
  }

  /// Returns an iterator over all entries in [FeelContext], in the order of their names: the
  /// order that two contexts with the same names share, whatever order they were written in.
  pub fn iter(&self) -> impl ExactSizeIterator<Item = (&Name, &Value)> {
    self.entries().iter()
  }

  /// Returns an iterator over all entries in [FeelContext], in the order they were written
  /// in: the order a context is shown in. An entry that was added later comes last, and an
  /// entry that was replaced keeps its place.
  pub fn iter_written(&self) -> impl ExactSizeIterator<Item = (&Name, &Value)> {
    self.entries().iter_written()
  }

  /// Returns an iterator over the names of the entries from the first name not less than `name` on, in their order.
  pub fn keys_from(&self, name: &str) -> impl Iterator<Item = &Name> {
    self.entries().names_from(name)
  }

  /// Returns a first value contained by context.
  pub fn get_first(&self) -> Option<&Value> {
    self.entries().values().next()
  }

  /// Returns the number of entries in [FeelContext].
  pub fn len(&self) -> usize {
    self.entries().len()
  }

  /// Returns `true` if [FeelContext] is empty.
  pub fn is_empty(&self) -> bool {
    self.entries().is_empty()
  }

  /// Returns `true` if [FeelContext] contains an entry with specified name, and the value is context.
  pub fn is_context(&self, name: &Name) -> bool {
    matches!(self.find(name), Some(Value::Context(_)))
  }

  /// Adds the entries of `other`, in the order they were written in: an entry of a name that
  /// this context has replaces it in its place, the others come last.
  pub fn zip(&mut self, other: &FeelContext) {
    for (name, value) in other.entries().iter_written() {
      self.put(name.clone(), value.clone());
    }
  }

  pub fn overwrite(&mut self, other: &FeelContext) {
    for (name, value) in other.entries().iter() {
      if self.entries().contains_key(name) {
        self.put(name.clone(), value.clone());
      }
    }
  }

  //TODO refactor the name, this operation is not moving, it is like prefixing
  pub fn move_entry(&mut self, name: Name, parent: Name) {
    if let Some(value) = self.remove_entry(&name) {
      self.create_entries(&[parent, name], value);
    }
  }

  /// Creates an entry with a value for specified [QualifiedName].
  /// All non-existing intermediary contexts will be created.
  pub fn create_entry(&mut self, qname: &QualifiedName, value: Value) {
    self.create_entries(qname.as_slice(), value);
  }

  /// Searches for a value of an entry pointed by specified qualified name.
  pub fn search_entry<'a>(&'a self, qname: &'a QualifiedName) -> Option<&'a Value> {
    self.search_deep(qname.as_slice())
  }

  /// Deep check for a value pointed by slice of names.
  pub fn contains_deep(&self, names: &[Name]) -> bool {
    if names.is_empty() {
      return false;
    }
    let tail = &names[1..];
    if let Some(value) = self.find(&names[0]) {
      if tail.is_empty() {
        return true;
      }
      if let Value::Context(context) = value {
        return context.contains_deep(tail);
      }
    }
    false
  }

  /// Creates entries with intermediary contexts when needed.
  pub fn create_entries(&mut self, names: &[Name], value: Value) {
    // if there are no entry names provided, then fo nothing
    if names.is_empty() {
      return;
    }
    let key = names[0].clone();
    let tail = &names[1..];
    // if tail is empty, then insert the value under the key in current context and return
    if tail.is_empty() {
      self.put(key, value);
      return;
    }
    // if there is a context under the key, then insert value to this context (a copy of it, when
    // it is shared, so only the contexts on the path are copied) and return
    if let Some(Value::Context(ctx)) = self.entries_mut().get_mut(&key) {
      Arc::make_mut(ctx).create_entries(tail, value);
      return;
    }
    // insert a value from tail to newly created context
    let mut ctx = FeelContext::default();
    ctx.create_entries(tail, value);
    self.put(key, ctx.into());
  }

  /// Sets `value` at the path `names` like [FeelContext::create_entries], with an error when an
  /// entry on the path holds a value that is not a context; the context is then left as it was.
  ///
  /// A path that would nest the value more levels deep than values may nest within an evaluation
  /// ([bound_depth](crate::diagnostics::bound_depth)) is refused before anything is built: the
  /// running evaluation stops, and the context is left as it was. A path that is blocked builds
  /// nothing, so it is the error of the blocking entry whatever its length.
  pub fn apply_entries(&mut self, names: &[Name], value: Value) -> Result<()> {
    // the part of the path that exists: no deeper than this context, so read without recursion
    let mut ctx: &FeelContext = self;
    for name in names.iter().take(names.len().saturating_sub(1)) {
      match ctx.entries().get(name) {
        Some(Value::Context(inner)) => ctx = inner,
        Some(other) => return Err(err_value_is_not_a_context(other)),
        None => break,
      }
    }
    if u64::try_from(names.len()).unwrap_or(u64::MAX).saturating_add(u64::from(value.nesting_depth())) > u64::from(diagnostics::MAX_VALUE_DEPTH) {
      diagnostics::too_deep();
      return Err(err_value_nested_too_deeply(names.len()));
    }
    self.create_entries(names, value);
    Ok(())
  }

  /// Deep search for a value pointed by names: the entry named by the first name, then in each
  /// context found the entry named by the next one. One step per name, without recursion, so the
  /// number of names is not bounded by the host stack.
  pub fn search_deep(&self, names: &[Name]) -> Option<&Value> {
    let (first, rest) = names.split_first()?;
    let mut value = self.find(first)?;
    for name in rest {
      let Value::Context(ctx) = value else {
        return None;
      };
      value = ctx.find(name)?;
    }
    Some(value)
  }

  /// The nesting depth of this context: one more than that of its deepest entry.
  pub(crate) fn nesting_depth(&self) -> u32 {
    let known = self.depth.load(Ordering::Relaxed);
    if known != 0 {
      return known;
    }
    let depth = self.entries().values().map(Value::nesting_depth).max().unwrap_or(0).saturating_add(1);
    self.depth.store(depth, Ordering::Relaxed);
    depth
  }

  /// The entries, to be changed: the nesting depth is forgotten.
  fn entries_mut(&mut self) -> &mut FeelContextEntries {
    self.entries();
    self.source = None;
    *self.depth.get_mut() = 0;
    self.built.get_mut().expect("the entries are read")
  }

  /// Inserts an entry.
  fn put(&mut self, name: Name, value: Value) {
    self.entries_mut().insert(name, value);
  }
}
