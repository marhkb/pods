#!/bin/sh
set -e

tag="$1"
out_dir="${tag}"
out_base_name="pods-${tag}"
tar_ball_name="${out_base_name}-${tag}.tar"
tar_ball_path="${out_dir}/${tar_ball_name}"

git show ${tag} --
mkdir ${out_dir}
git archive --format tar ${tag} > ${tar_ball_path}
mkdir "${out_dir}/${out_base_name}" && tar -xf ${tar_ball_path} -C "${out_dir}/${out_base_name}"
sh build-aux/dist-vendor.sh ../ "${out_dir}/${out_base_name}"

pushd ${out_dir}
mv .cargo vendor ${out_base_name}
pushd "${out_base_name}"
tar -czf "../${tar_ball_name}.xz" *
popd
sha256sum "${tar_ball_name}.xz" > "${tar_ball_name}.xz.sha256sum"
popd
