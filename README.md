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

Pods is a frontend for podman. It uses libadwaita for its user interface and strives to meet the design principles of GNOME.

With Pods you can, among other things:

- Connect to local and remote Podman instances.
- Easily overview images, containers and pods.
- View prepared information about images, containers, and pods.
- Inspect images, containers and pods.
- View and search container logs.
- Monitor processes of containers and pods.
- Download images and build them using Dockerfiles.
- Create pods and containers.
- Control the lifecycle of containers and pods (in bulk) (start, stop, pause, etc.).
- Delete images, containers, and pods (in bulk).
- Prune images.
- Rename containers.

## ⚡ Disclaimer

Pods is currently under heavy development. So be prepared for missing features and a lot of bugs.


## 🔌 Installation

The application is available ob Flathub Beta. First, add the the Flathub Beta remote
```shell
$ flatpak remote-add --if-not-exists flathub-beta https://flathub.org/beta-repo/flathub-beta.flatpakrepo
```
Then you can install the application by issuing
```shell
$ flatpak install flathub-beta com.github.marhkb.Pods
```
Pods can be kept up to date by issuing flatpak's update command like
```shell
$ flatpak update
```

You can also grab the latest CI build from [here](https://nightly.link/marhkb/pods/workflows/ci/main).
Then you need to unzip the archive's content and install the application with the command `flatpak install pods.flatpak`. Keep in mind


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
