# Android TV automation

This program allows me to power off my TV from my Linux box.

## The problem

My main PC is plugged in to my TV, and I'd very much like to avoid having to use
the TV's remote to turn it on every time I want to use my PC. The power button
on the remote doesn't work reliably (I suspect a bad switch), and the TV can't
be turned on for about 10 seconds after it has been turned off. The remote also
has a tendency to get lost in between sofa cushions.

There exists an HDMI feature called [CEC][cec] which allows TVs to power on/off
input devices, and likewise allows input devices to power on/off the TV.
However, I have an Nvidia graphics card, and [Nvidia doesn't support this
feature][nvidia] (on Linux?).

## The solution

My TV is a [Mi TV P1][tv], which runs Android TV. This means that it supports
debugging via [`adb`][adb], which I can use to inject a power button key event
to turn it off.

So far so good. Unfortunately, turning the TV off kills the `adb` connection, so
I need to use some other method to turn it on again. Luckily, it supports [Wake
on Wireless LAN][wol]. Using this, I can send it a magic packet over WiFi to
turn it on again.

But how does the program know when to turn the TV on or off? It can find out by
listening to signals sent by GNOME over [D-Bus][dbus]. When this is all put
together, it makes my TV act like any regular computer monitor.

## Usage

- [Install Rust][rustup].
- Install `adb` and some implementation of `ping`.
- Enable developer options on the TV by clicking on the build number in the
  Android settings a bunch of times.
- Enable wake on wireless network and USB debugging in the developer options
  (even though this uses WiFi)
- Set up a static IP address for the TV, either by assigning it a static DHCP
  lease, or by setting a static IP address in the TV's network settings.

Then install the program with `cargo install --path .`. This programs seems to
link to a bunch of dynamic libraries. You'll find out when building 🙂. Just
install the relevant `-dev` or `-devel` packages or whatever.

Look at the comments in [the systemd unit file](./tv-power.service) for
information on how to set this up as a systemd service.

[cec]: https://en.wikipedia.org/wiki/Consumer_Electronics_Control
[nvidia]: https://forums.developer.nvidia.com/t/hdmi-cec-support/31445
[tv]: https://www.mi.com/global/product/mi-tv-p1-55/
[adb]: https://developer.android.com/studio/command-line/adb
[wol]: https://en.wikipedia.org/wiki/Wake-on-LAN
[dbus]: https://www.freedesktop.org/wiki/Software/dbus/
[rustup]: https://rustup.rs/
