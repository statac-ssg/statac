# Can't listen

`statac serve` couldn't open the port it serves the site on. A port is a
number that tells apart the programs on one computer, and only one program
can listen on each port.

## What you'll see {#what-youll-see}

```text
error  Can't listen on port 3000: another program is using it
  Choose another port with `--port=<number>`.
  More: https://statac.dev/e/cant-listen/
```

## Why Statac stops

Without a port, a browser has nowhere to request the site from. If
another program is using the port, Statac tries the next ten ports before
it stops, and says which one it used. If the system refuses the port for
another reason, such as a port below 1024 that only an administrator may
open, Statac stops at once.

## How to fix it

Choose a free port with `--port`, or stop the program that is using the
port you want. Ports from 1024 to 65535 need no special permission.

## Example

On many systems, only an administrator may open port 80. Choose a port
that any program may open:

```text
statac serve --port=8080
```
