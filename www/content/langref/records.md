# Records

A _record_ is a value that associates some _fields_ with other values.

```roc

```

The fields

Currently our `addAndStringify` function takes two arguments. We can instead make it take one argument like so:

```roc
total = addAndStringify { birds: 5, iguanas: 7 }

addAndStringify = \counts ->
    Num.toStr (counts.birds + counts.iguanas)
```

The function now takes a _record_, which is a group of named values. Records are not [objects](<https://en.wikipedia.org/wiki/Object_(computer_science)>); they don't have methods or inheritance, they just store information.

The expression `{ birds: 5, iguanas: 7 }` defines a record with two _fields_ (the `birds` field and the `iguanas` field) and then assigns the number `5` to the `birds` field and the number `7` to the `iguanas` field. Order doesn't matter with record fields; we could have also specified `iguanas` first and `birds` second, and Roc would consider it the exact same record.

When we write `counts.birds`, it accesses the `birds` field of the `counts` record, and when we write `counts.iguanas` it accesses the `iguanas` field.

When we use [`==`](/builtins/Bool#isEq) on records, it compares all the fields in both records with [`==`](/builtins/Bool#isEq), and only considers the two records equal if all of their fields are equal. If one record has more fields than the other, or if the types associated with a given field are different between one field and the other, the Roc compiler will give an error at build time.

> **Note:** Some other languages have a concept of "identity equality" that's separate from the "structural equality" we just described. Roc does not have a concept of identity equality; this is the only way equality works!

### [Accepting extra fields](#accepting-extra-fields) {#accepting-extra-fields}

The `addAndStringify` function will accept any record with at least the fields `birds` and `iguanas`, but it will also accept records with more fields. For example:

```roc
total = addAndStringify { birds: 5, iguanas: 7 }

# The `note` field is unused by addAndStringify
totalWithNote = addAndStringify { birds: 4, iguanas: 3, note: "Whee!" }

addAndStringify = \counts ->
    Num.toStr (counts.birds + counts.iguanas)
```

This works because `addAndStringify` only uses `counts.birds` and `counts.iguanas`. If we were to use `counts.note` inside `addAndStringify`, then we would get an error because `total` is calling `addAndStringify` passing a record that doesn't have a `note` field.

### [Record shorthands](#record-shorthands) {#record-shorthands}

Roc has a couple of shorthands you can use to express some record-related operations more concisely.

Instead of writing `\record -> record.x` we can write `.x` and it will evaluate to the same thing: a function that takes a record and returns its `x` field. You can do this with any field you want. For example:

```roc
# returnFoo is a function that takes a record
# and returns the `foo` field of that record.
returnFoo = .foo

returnFoo { foo: "hi!", bar: "blah" }
# returns "hi!"
```

Sometimes we assign a def to a field that happens to have the same name—for example, `{ x: x }`.
In these cases, we shorten it to writing the name of the def alone—for example, `{ x }`. We can do this with as many fields as we like; here are several different ways to define the same record:

- `{ x: x, y: y }`
- `{ x, y }`
- `{ x: x, y }`
- `{ x, y: y }`

### [Record destructuring](#record-destructuring) {#record-destructuring}

We can use _destructuring_ to avoid naming a record in a function argument, instead giving names to its individual fields:

```roc
addAndStringify = \{ birds, iguanas } ->
    Num.toStr (birds + iguanas)
```

Here, we've _destructured_ the record to create a `birds` def that's assigned to its `birds` field, and an `iguanas` def that's assigned to its `iguanas` field. We can customize this if we like:

```roc
addAndStringify = \{ birds, iguanas: lizards } ->
    Num.toStr (birds + lizards)
```

In this version, we created a `lizards` def that's assigned to the record's `iguanas` field. (We could also do something similar with the `birds` field if we like.)

Finally, destructuring can be used in defs too:

```roc
{ x, y } = { x: 5, y: 10 }
```

### [Making records from other records](#making-records-from-other-records) {#making-records-from-other-records}

So far we've only constructed records from scratch, by specifying all of their fields. We can also construct new records by using another record to use as a starting point, and then specifying only the fields we want to be different. For example, here are two ways to get the same record:

```roc
original = { birds: 5, zebras: 2, iguanas: 7, goats: 1 }
fromScratch = { birds: 4, zebras: 2, iguanas: 3, goats: 1 }
fromOriginal = { original & birds: 4, iguanas: 3 }
```

The `fromScratch` and `fromOriginal` records are equal, although they're defined in different ways.

- `fromScratch` was built using the same record syntax we've been using up to this point.
- `fromOriginal` created a new record using the contents of `original` as defaults for fields that it didn't specify after the `&`.

Note that `&` can't introduce new fields to a record, or change the types of existing fields.
(Trying to do either of these will result in an error at build time!)

## [Low-level details](#low-level) {#low-level}

The empty record (`{}`) holds no information, and takes up no memory at runtime. Records where one field takes up memory get "unwrapped" automatically; at runtime, it's exactly as if there were no record surrounding the field at all. For example, a `{ blah: Str }` record has a runtime representation that's exacly the same as [`Str`](Str#Str). The same is true of `{ blah: Str, emptyRecord: {} }` because the `emptyRecord` field has a type that takes up no memory; this would also be represented as a standalone [`Str`](Str#Str) at runtime.

Records with multiple fields are represented at runtime as [C structs](https://en.wikipedia.org/wiki/Struct_(C_programming_language)). This means that the field name strings are not stored in the record at runtime, and the values are stored next to each other in memory.

### [Field ordering at runtime](#field-ordering) {#field-ordering}

Since Roc records are anonymous, the compiler uses the following algorithm to decide which order to put the fields in:

1. Sort the fields by their values' [alignments](https://en.wikipedia.org/wiki/Data_structure_alignment)
2. Sort fields with the same alignments alphabetically by field name.

For example, consider this record:

```roc
Account : {
    firstName : Str,
    middleName : Str,
    lastName : Str,
    age : U8,
    points : I16,
}
```

First, it will be sorted by alignment. `U8` always has an alignment of 1, and `I16` always has an alignment of 2. The alignment of [`Str`](Str#Str) varies based on the compilation target; for 32-bit targets it has an alignment of 4, and for 64-bit targets it has an alignment of 8. Either way, [`Str`](Str#Str)'s alignment is the highest, followed by `I16` and then `U8`, so the fields will be sorted in this order based on alignment:

* `firstName : Str`
* `lastName : Str`
* `middleName : Str`
* `points : I16`
* `age : U8`

> Notice that `age` is at the end of the list, even though in the `Account` type alias, `points` was last. Record field order type aliases (and opaque types) does not affect runtime representation at all. All that matters is the field names and the alignment of their contents. This is necessary so that anonymous records (used without giving them a type alias or opaque type name) will have the same in-memory representation at runtime as records which happened to have been named.

Next, fields with the same alignment will be sorted alphabetically by field name. Since three fields have the type [`Str`](Str#Str), they all have the same alignment, and will be sorted alphabetically to produce this final field ordering:

* `firstName : Str`
* `lastName : Str`
* `middleName : Str`
* `points : I16`
* `age : U8`

> Again, notice that `middleName` now appears after `firstName` and `lastName`, even though that's not where it was defined in the `Account` type alias. That's because `m` is sorted alphabetically after `l`.

### [Records nested inside other records](#nested) {#nested}

Records inside records are "flattened"—for example:

```roc
Record : {
    one : { a : Str, b : Str },
    two : { c : Str, d : Str },
}
```

At runtime, this will be four strings side by side in memory.

Note that the contents of each record will still be adjacent. For example:

```roc
Record : {
    one : { a : U32, b : U16, c : U16 },
    two : { d : U32, e : U16, f : U16 },
}
```

In memory, this record will *not* be two `U32`s followed by four `U16`s. That would have required splitting up the records, which Roc's compiler does not do. Instead, it puts them side by side; the in-memory representation of this whole record will be a `U32` followed by two `U16`s, and then another `U32` followed by another two `U16`s.

### [Alignment and padding](#alignment) {#alignment}

The [alignment](https://en.wikipedia.org/wiki/Data_structure_alignment) of a record is equal to the highest alignment among all of its fields.

Since CPUs access memory most efficiently when memory reads are aligned, Roc's compiler (like many) inserts [data padding](https://en.wikipedia.org/wiki/Data_structure_alignment#Data_structure_padding) when necessary to make sure reading any particular field will be aligned optimally.

For example, the `Account` type from earlier takes up 75 bytes in memory on a 64-bit target (each of the three [`Str`](Str#Str) values takes up 24 bytes on a 64-bit target, `I16` takes 2, and `U8` takes 1). Suppose we had a record which stored two of them:

```
TwoAccounts : {
   one : Account,
   two : Account,
}
```

Without alignment padding, this would take up 150 bytes in memory because the second `Account` would be placed immediately after the first `Account`. However, this would result in all of the [`Str`](Str#Str)s of the second `Account` being unaligned because they were placed right after the 75 bytes of the first `Account`, and if you add 75 to a memory address that's aligned to a multiple of 8, you the resulting address will not be a multiple of 8.

To fix this, Roc's compiler will automatically insert 5 bytes of "padding" (unused memory that will be ignored) after the end of the first record, so that the second record will begin 80 bytes after the first one instead of 75 bytes. This will result in all the reads for the second record being aligned.

This padding can compound; if you have a `List TwoAccounts`, there will be an extra 5 bytes of padding (unused memory) for each element in that list. One way to avoid padding costs accumulating like this is to use a [struct-of-arrays](https://en.wikipedia.org/wiki/AoS_and_SoA) representation instead of a list-of-structs representation like `List TwoAccounts`.

### [Closures in records](closures) {#closures}

Roc functions are closures, and the values they capture can have different alignments. At runtime, captures are represented as records, and so a function inside a record is essentially a record inside a record at runtime.

```roc
Record : {
    number : U32,
    doSomething : U16 -> U16,
    byte : U8,
}
```

Looking at the field `function : U16 -> U16`, it might be tempting to assume its alignment would be based on its argument or return types. However, its alignment has absolutely nothing to do with the type of the function!

That `doSomething` field stores the function's captured values, and so its alignment depends on what was actually captured. (See [function low-level details](functions#low-level) for how the size and alignment of function captures are calculated.)

Since the capture's alignment can vary based on what actually gets captured, that in turn means that the ordering of the fields in this record at runtime can change depending on what `doSomething` captures.

For example, if `doSomething` captures a value with an alignment of 1, it may end up being ordered after the `byte : U8` field (because they both have an alignment of 1, and `d` comes after `b` alphabetically). However, if `doSomething` captures something with an alignment of 2, then it will end up betewen the `byte : U8` and `number : U32` fields. If it captures something with an alignment of 8, then it will be ordered at the very beginning of the record.
