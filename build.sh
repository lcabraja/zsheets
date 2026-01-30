#!/bin/bash
set -e

APP_NAME="zsheets"
APP_BUNDLE="${APP_NAME}.app"
INSTALL_DIR="$HOME/Applications"

echo "Building ${APP_NAME}..."
cargo build --release

echo "Creating app bundle..."
rm -rf "${APP_BUNDLE}"
mkdir -p "${APP_BUNDLE}/Contents/MacOS"
mkdir -p "${APP_BUNDLE}/Contents/Resources"

cp "target/release/${APP_NAME}" "${APP_BUNDLE}/Contents/MacOS/"
cp "Info.plist" "${APP_BUNDLE}/Contents/"
echo "APPL????" > "${APP_BUNDLE}/Contents/PkgInfo"

echo "Installing to ${INSTALL_DIR}..."
mkdir -p "${INSTALL_DIR}"
rm -rf "${INSTALL_DIR}/${APP_BUNDLE}"
cp -r "${APP_BUNDLE}" "${INSTALL_DIR}/"

echo "Done! ${APP_NAME} installed to ${INSTALL_DIR}/${APP_BUNDLE}"
