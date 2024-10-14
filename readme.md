# Simple Request Blocker

## What is this?

This is a lightweight application built using components such as Hyper-RS and Tokio to create a man-in-the-middle style proxy service.

This service sits on your local machine and can be activated on the designated port.

It offers the ability to provide either an Allow list or Deny list to block or allow outgoing traffic - great for monitoring Advertisements or blocking specific services. These lists are customisable and hot-swappable.

A logger is provided to view the previous requests and whether they've been blocked or not.

### How to use?

Simply start the service on the desired port and choose whether to enable blocking or not.

Point any application you wish to run through the service by using the IP & PORT.

Once you view requests passing through the service, you can choose to add specific requests to an exclusion list.

There are 2 options of exclusions - depending on the chosen solution you are given either an allow list or a deny list.

With the allow list, requests are allowed by default and only items in the exclusion list are blocked.

With the deny list, requests are blocked by default and only items in the exclusion list are allowed.

> **Example:** The `Allow Incoming` list could contain _google.com_ and when in use, only addresses containing variations of _google.com_ are allowed.

Eventually you will be able to Import/Export these lists into CSV - by default, previous sessions are saved so lists and settings remain the same between application runs.

### Issues and Feature tracking:

Please check the TODO file for more information on planned features and known issues.

The TODO file can be found [here](TODO.md).
