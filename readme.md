# Simple Request Blocker

## What is this?

This is a lightweight application built using components such as Hyper-RS and Tokio to create a man-in-the-middle style proxy service.

This service sits on your local machine and can be activated on the designated port.

It offers the ability to provide either an Allow list or Deny list to block or allow outgoing traffic - great for monitoring Advertisements or blocking specific services. These lists are customisable and hot-swappable.

A logger is provided to view the previous requests and whether they've been blocked or not.

### How to use?

Simply create `allow_list.csv` & `block_list.csv` files with a title column named _uri_ and then a list of Strings containing the exclusions.

> **Example:** `allow_list.csv` could contain _google.com_ and when in use, only addresses containing variations of _google.com_ are allowed.

Or, you can wait until the requests start piling up and you can dynamically add addresses to the exclusion lists - these can be exported (eventually).

### Feature Plans:

The following are my currently designated tasks, they may not be completed in order.

- [] Add dynamic filtering and requests logs.
- [] Create dynamic allow & deny lists.
- [] Allow exports of allow & deny lists.
- [] Maybe enable remote running & certificate allocation.
- [] Add ability to run as a non-GUI application (use flags?).
- [] Allow dragging?
- [] Window Resizing?
- [] Logging Filtering?
- [] State Saving (the rest of it). [^1]

### Known Issues:

- State doesn't save correctly.
- Input box for Port & Start Proxy button aren't completely aligned.

### Other Notes:

[^1]: Some state is saved, however, due to Mutex not working correctly, some values no longer change after state recovery.
