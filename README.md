# Studio-4T

This is open-source alternative to [Studio-3T](https://studio3t.com/). The project is being built with [Tauri](https://tauri.app/). And I am using [Vue.js](https://vuejs.org/) for front-end. And it is not ready yet.

![Current state of development](img/Screenshot%20from%202025-04-12%2013-11-48.png)

# Contents

- [Why?](#why)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Usage](#usage)
- [Want to Contribute?](#want-to-contribute)

### Why?
I really love Studio-3T however the features I really like are only included in their subscription models(Basic/Pro/Ultimate). And I have tried to find comparable open-source alternative to Studio-3T, surprisingly, I have not found anything. And that is the reason why I am developing Studio-4T in order to give back to community. When I started this project, I wanted to learn Rust because - 'why not?'. And I have a specific problem and the technology I want to learn. Bingo! 

I wanted a tool that allows you to (plus what Studio-3T provides):
- Open-source and free.
- Light-weight and fast startup time.
- Not buggy(in my fedora work laptop with multi-monitor setup it very often just freezes without any feedback).
- More customizeable.

`Studio-4T` will check all of those boxes in the future for me.

### Prerequisites

---
1) First of all, please follow instructions [tauri prerequisites](https://tauri.app/start/prerequisites/) and make sure that you have installed platform-specific system dependencies. They have awesome guides for major platforms (kudos!).
2) Make sure that you have installed `rust` and `node`. [Instructions](https://tauri.app/start/prerequisites/#rust).


### Installation

---
Currently there are not any pre-built binaries which you just download and run. I am going to release binaries when I finish implementing basic database manager functionalities. So for now you have to build your own ones in order to test it :).


In order to locally build: 

1) `npm install`
2) `npm run tauri dev` and the window should pop up.



---
What I am currently working on:
`Database connection`