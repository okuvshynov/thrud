Let's build a simple system metric collector for local/small-scale networking use.

It will be somewhere in between fully local performance monitoring tools (atop, htop, etc.) and large-scale frameworks (like prometheus).

Important considerations:
- support for MacOS and Linux, eventually maybe Windows
- built-in support for GPU monitoring - NVidia, AMD, Intel, Apple
- ~easy to extend with new metrics (say, swap or memory)
- eventually can be used in multiple ways: 1) data collection and writing to local sqlite db (for historical queries, similar to atop), http endpoint to return the history, command-line TUI app to explore history + current data, ability to connect from that app to remote machine running it).
- I'm thinking about using Rust and use bindings to query platform specific things (for example, see swift prrof of concept code for Apple Silicon CPU and GPU monitoring)

We need to have layered architecture:
1. Collectors. These are platform specific and metric specific. Each collector produces a set of metrics. Each metric can be seen as value with associated metadata dictionary. For example, cpu_load = {'value': 0.45, timestamp: 123321, 'metadata': {'core': 1, 'core_type': 'efficiency', 'numa_node': 0}}. At this layer collectors should be stateless. For example, if we get total number of bytes read from network interface, we export total bytes. If to compute rate per second, we'll need to take historical data and divide by interval, that should be done at a different level.
2. Raw data saving: collector output should be written to local sqlite database (say, ~/.thrud/thrud.db by default). 
3. Metric transforms: this is how we can get things like:
   - disk read rate
   - aggregated load per cpu socket/cluster
   - total power consumption (from components)
   Ideally we should be able to do that by writing SQL queries over the data.
4. Interfaces:
    - http endpoint to serve metrics
    - command-line TUI app
    - maybe more

Let's keep this in mind, but start with small set of metrics and platforms, specifically do steps (1) and (2) for the Apple Silicon CPU and GPU monitoring.

