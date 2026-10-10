# Can't listen

`statac serve` couldn't open the port it shows the site on. A port is a
number that tells the programs on one computer apart, and only one program
can listen on each.

## What you'll see {#what-youll-see}

```text
error  Can't listen on port 3000: another program is using it
  Choose another port with `--port=<number>`.
  More: https://statac.dev/e/cant-listen/
```

## Why Statac stops

With no port there is nowhere for a browser to ask for the site. Where
another program is using the port, Statac tries the ten after it before it
stops, and says which one it took. Where the system refuses for another
reason, such as a port below 1024 that only an administrator may open,
Statac stops at once.

## How to fix it

Name a port that is free with `--port`, or stop the program that is using
the one you want. Ports from 1024 to 65535 need no special permission.

## Example

On many systems, only an administrator may open port 80. Choose a port any
program may open:

```text
statac serve --port=8080
```
