# amsg-batch

![Version](https://img.shields.io/badge/version-v0.1.0--dev-pink)
![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-rebeccapurple)
[![Build Status](https://github.com/Luis-Varona/amsg-batch/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Luis-Varona/amsg-batch/actions/workflows/ci.yml?query=branch%3Amain)

## Overview

`amsg-batch` is a command-line tool to send bulk texts via Apple Messages on macOS, written in Rust. A friend of mine who runs a business originally asked me to help him send out personalized batches of messages from his iPhone number to clients, which gave birth to this idea.

Due to widespread and well-known difficulties integrating with Apple's API from non-Apple products, this CLI (which relies heavily on AppleScript) only works on macOS; this is a limitation of Apple itself, not the tool's implementation. In the end, it is aimed at those who need to send out bulk texts from iPhone numbers, anyway.

## Installation

[TODO: Write here once first release is out and `install.sh` is finalized]

## Basic use

Let us say you wish to send a personalized message to several of your friends via iMessage. Simply create a CSV file (say, `recipients.csv`) with two columns

```csv
Baron von Murderpillow,+1 (234) 567-8910
Rt. Hon. John A. Stymers,314159265
[...]
```

and a text file (say, `message.txt`) with contents

```text
Greetings and salutations, my dearest {name}! I doth send to thee a most wondeful message.
```

(as you can see, the formatting of the phone numbers does not matter, as long as they are valid). Then, assuming you have `amsg-batch` properly installed, simply run

```bash
amsg-batch --recipients recipients.csv --message message.txt --placeholder {name}
```

which will replace `{name}` with the names in the CSV file and send the personalized messages to the corresponding phone numbers with a one-second delay between each text.

You may also wish to send non-personalized messages to a list of phone numbers, and perhaps via SMS instead of iMessage. To do this, simply create a CSV file (say, `recipients.csv`) with a single column

```csv
+1 (234) 567-8910
314159265
[...]
```

and a text file (say, `message.txt`) with contents

```text
Greetings and salutations, my dearest friend! I doth send to thee a most wondeful message.
```

then run

```bash
amsg-batch --recipients recipients.csv --message message.txt --service SMS
```

to send the same message to all phone numbers via SMS.

## Documentation

To see brief descriptions of all available options, run

```bash
amsg-batch -h
```

(assuming you have `amsg-batch` properly installed). To see more detailed descriptions and instructions, run

```bash
amsg-batch --help
```

A GitHub Pages site with more extensive documentation generated with `cargo doc` is forthcoming.
