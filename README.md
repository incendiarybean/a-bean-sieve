# A Bean Sieve

## What is a Bean Sieve?

The expectation is that if you sieve a bowl of beans and liquid, you'll be left with only the beans.

This is a lightweight application built using components such as EGUI, Hyper-RS and Tokio to create a man-in-the-middle style proxy service.

You can start this service on any device and assign it a port, then set the assigned address:port as your proxy.

The UI offers the capability to build dynamnic Allow/Deny lists based on incoming/outgoing traffic.

You can export/import the Allow/Deny lists using the UI, so they can be saved for later - the service does, however, save state between runs.

You can view the logs of both the service and requests within the UI. You can also use a custom logging level to remove the unnecessary bloat of day-to-day logging.

## Installation

### From the Source Code

Clone the repository to a local destination, navigate to the folder in the CLI.

Start the build:
```bash
cargo run build --release
```

Navigate to the exectuable directory:
```bash
cd target/release
```

Move your desired executable to wherever you want, or run it directly from the folder:
```bash
./a-bean-sieve.exe
```

### From the executable
Download the released executable for your system.

Move your executable to wherever you want, or run it directly from the folder:
```bash
./a-bean-sieve.exe
```

## Usage

Simply start the service, enter the desired port into the UI and choose whether to enable Proxy Filtering or not.

Point any application you wish to run through the service by using the assigned ADDRESS:PORT (by default binding to 127.0.0.1:PORT).

Once you view requests passing through the service, you can choose to add specific requests to an exclusion list.

### Standard Usage

Without adding an exclusions, you can still monitor all requests being made through the proxy, including the status of these requests.

You can export any of your configurations into a CSV file that can be used to re-import these settings later.

Currently supported exports:

- Requests List
- Allow List
- Deny List

By default, while using the Application with its UI, the UI will store previous state - so you will not need to export/re-import everytime you use the application.

### Exclusions
There are 2 options of exclusions:

- [Deny](#deny)
- [Allow](#allow)

#### Deny
Deny means that all requests will be denied by default.

Choosing Deny will create an `Allow List`.

With the `Allow List`, requests are blocked by default and only items added to this exclusion list are allowed.

> **Example:** The `Allow List` list could contain *google.com* and when in use, only addresses containing variations of *google.com* are allowed.

#### Allow
Allow means that all requests will be allowed by default.

Choosing Allow will create an `Deny List`.

With the `Deny List`, requests are allowed by default and only items added to this exclusion list are blocked.

> **Example:** The `Deny List` list could contain *google.com* and when in use, only addresses containing variations of *google.com* are denied.

## Issues and Feature tracking:

Please check the TODO file for more information on planned features and known issues.

The TODO file can be found [here](TODO.md).
