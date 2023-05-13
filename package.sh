#!/bin/sh

echo Building docker image...
docker build -t lichen --load . || exit $?
docker save lichen | gzip > lichen_docker.tar.gz

echo Building AppImage...
mkdir -p AppDir
cp assets/* AppDir/
docker run -v $PWD/AppDir:/AppDir --rm --entrypoint sh lichen -c "cp /lichen /AppDir/" || exit $?

appimagetool AppDir || exit $?
mv lichen-x86_64.AppImage lichen.AppImage

echo Done