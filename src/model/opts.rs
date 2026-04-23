use gtk::glib;

use crate::engine;
use crate::monad_boxed_type;

monad_boxed_type!(pub(crate) BoxedContainerCreateOpts(engine::opts::ContainerCreateOpts) impls Default);
monad_boxed_type!(pub(crate) BoxedContainerCreateVolumeOpts(engine::opts::ContainerCreateVolumeOpts) impls Default is nullable);
monad_boxed_type!(pub(crate) BoxedContainersPruneOpts(engine::opts::ContainersPruneOpts) impls Default);
monad_boxed_type!(pub(crate) BoxedImagesPruneOpts(engine::opts::ImagesPruneOpts) impls Default);
monad_boxed_type!(pub(crate) BoxedImagePullOpts(engine::opts::ImagePullOpts) impls Default);
monad_boxed_type!(pub(crate) BoxedVolumesPruneOpts(engine::opts::VolumesPruneOpts) impls Default);
