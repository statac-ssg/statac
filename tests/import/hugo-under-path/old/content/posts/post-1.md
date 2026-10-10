---
title: Post 1
date: 2026-01-01
---
A post linking to others by ref and relref, in a site under a path.

- [Post 2]({{< ref "post-2.md" >}})
- [Post 2, relatively]({{< relref "post-2" >}})
- [About]({{< relref "/about.md#team" >}})
- [This place]({{< relref "#notes" >}})
- [Post 9]({{< ref "post-9.md" >}})
- [About, after the site's address]({{< baseurl >}}about/)
- [Post 2, after the site's address]({{< baseurl >}}{{< relref "post-2" >}})
- [Post 2, wholly, after the site's address]({{< baseurl >}}{{< ref "post-2.md" >}})

## Notes

The heading a link on this page lands on.

{{< figure src="/blog/images/photo-1.jpg" alt="A picture named with the path the site is under" >}}
