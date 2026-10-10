Before the call.

@box('a')
Inside the call, *read as Markdown*.

- One
- Two
@end

Words before.
@box()
Close against the words.
@end
Words after.

> A quotation.
>
> @box()
> Inside the quotation.
>
> More inside.
> @end

- An item.

  @box()
  Inside the item.

      code inside
  @end

- A tight item.
  @box()
  Inside the tight item.
  @end

@box('outer')
Outer.

@box('inner')
## Inner heading
@end
@end

```text
@box()
Shown as code.
@end
```

<div>
@box()
Inside *HTML*.
@end
</div>
