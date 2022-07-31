<h1 align="center">
  Pods
</h1>

<p align="center"><strong>A Podman desktop application</strong></p>

<p align="center">
  <a href="https://hosted.weblate.org/engage/pods/">
    <img src="https://hosted.weblate.org/widgets/pods/-/pods/svg-badge.svg" alt="Translation status" />
  </a>
  <a href="https://github.com/marhkb/pods/actions/workflows/ci.yml">
    <img src="https://github.com/marhkb/pods/actions/workflows/ci.yml/badge.svg" alt="CI status"/>
  </a>
</p>

<br>

<p align="center">
  <img src="data/resources/screenshots/preview.png" alt="Preview"/>
</p>

Interact with Podman using an intuitive desktop application.

Pods focuses on simplicity and good usability.
The only requirement is that you have Podman installed.


## ⚡ Disclaimer

Pods is currently under heavy development. So be prepared for missing features and a lot of bugs.


## 🔌 Installation

You can grab the latest CI build from [here](https://nightly.link/marhkb/pods/workflows/ci/main).
Then you need to unzip the archive's content and install the application with the command `flatpak install pods.flatpak`.


## 🏗️ Building from source

### GNOME Builder

GNOME Builder is the environment used for developing this application.
It can use Flatpak manifests to create a consistent building and running
environment cross-distro. Thus, it is highly recommended you use it.

1. Download [GNOME Builder](https://flathub.org/apps/details/org.gnome.Builder).
2. In Builder, click the "Clone Repository" button at the bottom, using `https://github.com/marhkb/pods.git` as the URL.
3. Click the build button at the top once the project is loaded.

### Meson

#### Prerequisites

The following packages are required to build Pods:

* meson
* ninja
* appstream-glib (for checks)
* cargo
* glib2
* gtk4
* libadwaita >= 1.2

#### Build Instruction

```shell
git clone https://github.com/marhkb/pods.git
cd pods
meson _build --prefix=/usr/local
ninja -C _build install
```


## 🙌 Help translate Pods

You can help Pods translate into your native language. If you found any typos
or think you can improve a translation, you can use the [Weblate](https://hosted.weblate.org/engage/pods/) platform.


## 💝 Acknowledgment

The library [podman-api-rs](https://github.com/vv9k/podman-api-rs) provides a rust interface to the Podman API.
Without this great project, Pods would probably never have come into existence.

I also wanted to thank [SeaDve](https://github.com/SeaDve), from whom I took the [gettext](https://github.com/SeaDve/scripts/blob/0bd6f162ec8f2b3f0a9ad12816477fed934077db/gettext_rs.py) python script and whose projects like [Kooha](https://github.com/SeaDve/Kooha) and [Mousai](https://github.com/SeaDve/Mousai) served as inspiration for the README.

And also, a warm thank you to all the [contributors](https://github.com/marhkb/pods/graphs/contributors)
and [translators](https://hosted.weblate.org/engage/pods/) from Weblate.
