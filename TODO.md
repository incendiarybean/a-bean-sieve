# TODOs, Features & Issues

These are lists of features likely to be added and known issues likely to be investigated.

These list are mostly for personal reference so I can remember what I wanted to complete.

## Features
    - [x] Add dynamic filtering and requests logs.
    - [x] Create dynamic allow & deny lists.
    - [x] Allow exports of allow & deny lists.
    - [x] Allow imports of allow & deny lists.
    - [ ] Re-enable Drag & Drop support
    - [x] Rework Event handling, e.g. separate ::new/::default and Self::event_handler
    - [x] Rework proxy_start
    - [ ] ~~Add logging event~~ - **WON'T DO** [^2]
    - [x] Remove printlns to opt for logger
    - [x] Add logs panel
    - [x] Allow users to change log_level
    - [ ] Make logs filterable e.g. log_level
    - [ ] Make exclusion list editor better
    - [ ] Add friendly icons
    - [ ] Reword/iconise the expand button
    - [ ] HTTP1/HTTP2 switch
    - [ ] HTTPS support?
    - [x] State Saving (the rest of it). [^1]
    - [ ] Enable CLI only flags

## Issues
    - [x] Input box for Port & Start Proxy button aren't completely aligned.

## Notes

[^1]: Required state is saved, e.g. TrafficFilter, port, exclusion lists...

[^2]: Creating a logger struct and assigning this to the Proxy struct has made an easier approach to logging, and has meant cutting down on the event handler's required events.
