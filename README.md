<h1 align="center">
  Symphony
</h1>

<p align="center"><strong>A Podman desktop application</strong></p>

<p align="center">
  <a href="https://hosted.weblate.org/engage/symphony/">
    <img src="https://hosted.weblate.org/widgets/symphony/-/symphony/svg-badge.svg" alt="Translation status" />
  </a>
  <a href="https://github.com/marhkb/symphony/actions/workflows/ci.yml">
    <img src="https://github.com/marhkb/symphony/actions/workflows/ci.yml/badge.svg" alt="CI status"/>
  </a>
</p>

<br>

<p align="center">
  <img src="data/resources/screenshots/preview.png" alt="Preview"/>
</p>

Interact with Podman using an intuitive desktop application.

Symphony focuses on simplicity and good usability.
The only requirement is that you have Podman installed.


## ⚡ Disclaimer

Symphony is currently under heavy development. So be prepared for missing features and a lot of bugs.


## 🔌 Installation

You can grab the latest CI build from [here](https://nightly.link/marhkb/symphony/workflows/ci/main/symphony-x86_64.zip).
Then you need to unzip the archive's content and install the application with the command `flatpak install symphony.flatpak`.


## 🏗️ Building from source

### GNOME Builder

GNOME Builder is the environment used for developing this application.
It can use Flatpak manifests to create a consistent building and running
environment cross-distro. Thus, it is highly recommended you use it.

1. Download [GNOME Builder](https://flathub.org/apps/details/org.gnome.Builder).
2. In Builder, click the "Clone Repository" button at the bottom, using `https://github.com/marhkb/symphony.git` as the URL.
3. Click the build button at the top once the project is loaded.

### Meson

#### Prerequisites

The following packages are required to build Symphony:

* meson
* ninja
* appstream-glib (for checks)
* cargo
* glib2
* gtk4
* libadwaita

#### Build Instruction

```shell
git clone https://github.com/marhkb/symphony.git
cd symphony
meson _build --prefix=/usr/local
ninja -C _build install
```


## 🙌 Help translate Symphony

You can help Symphony translate into your native language. If you found any typos
or think you can improve a translation, you can use the [Weblate](https://hosted.weblate.org/engage/symphony/) platform.
