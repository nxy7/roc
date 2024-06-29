
## [Low-level details](#low-level) {#low-level}

Note that separate tag payloads are reprsented the same way as [tuples](tuples#low-level). For example, there is no difference at runtime between `[Something Str Str]` and `[Something (Str, Str)]`.

Tag unions are named after [tagged unions](https://en.wikipedia.org/wiki/Tagged_union), which is how they are represented in memory by default. (Certain special cases get optimized differently, which the next section will discuss.) For example, let's consider this tag union:

```roc
[Foo Str, Bar (Str, Str), Baz U8]
```

In memory, this will be represented as essentially the following:

```roc
{
    discriminant : U8,
    payload : (Str, Str)
}
```

The discriminant is an integer which tells us whether we have `Foo`, `Bar`, or `Baz`. It will almost always be [`U8`](Num#U8) (unless the tag union somehow has more than 256 tags).

The payload field's type is determined by the largest of the different payload types in the tag union—in this case, `(Str, Str)`—since if there's enough memory available to store the largest one, there's necessarily enough memory to store the smaller ones as well.

Only one payload at a time will ever be stored in this field, and so sometimes there will be extra unused memory left over. This is unavoidable; the tag union always has to reserve enough space for the largest payload that could be stored there, because there's no way to know at compile time which of the payloads will end up needing to be stored there at runtime!

This means it can be advantageous for performance to keep the payload sizes similar in size. For example, if one of the tags has a payload that's much bigger than the others, and that tag is rarely created in comparison to the others, it might be advantageous to wrap its payload in a [`Box`](Box#Box). By default, wrapping in a [`Box`](Box#Box) hurts performance, but making all the other payloads take up less space could make it advantageous overall in this case.

Note that the payload field can be empty. For example:

```roc
[Foo, Bar, Baz]
```

This gets compiled to a [`U8`](Num#U8) because the `payload` field is empty, leaving the discriminant as the only field in the record that takes up any space, which in turn means the surrounding record gets optimized away completely.

### Optimized special cases

Sometimes we can do better than this "tagged union" representation. For example, the empty tag union (`[]`) holds no information, and takes up no memory at runtime. The same is true of tag unions with one tag and no payload; the tag union `[Blah]` holds no information and takes up no memory at runtime.

Tag unions with one tag and a payload get "unwrapped" automatically; at runtime, it's exactly as if there were no tag union surrounding the value at all. For example, a `[Foo Str]` tag union has a runtime representation that's exacly the same as [`Str`](Str#Str).

Tag unions with multiple tags and no payloads are represented at runtime as unsigned integers. For example, the tag union `[A, B, C]` is represented as a [`U8`](Num#U8) at runtime. If the union has more than 256 elements, it will be represented at runtime as a [`U16`](Num#U16) instead of [`U8`](Num#U8).
