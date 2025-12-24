#!/bin/bash
# https://raw.githubusercontent.com/rust-lang/rust/master/src/ci/docker/scripts/musl-toolchain.sh
# This script runs `musl-cross-make` to prepare C toolchain (Binutils, GCC, musl itself)
# and builds static libunwind that we distribute for static target.
#
# Versions of the toolchain components are configurable in `musl-cross-make/Makefile` and
# musl unlike GLIBC is forward compatible so upgrading it shouldn't break old distributions.
# Right now we have: Binutils 2.31.1, GCC 9.2.0, musl 1.2.3.

# ignore-tidy-linelength

set -ex
set -o pipefail

cpu_count() {
  local n
  n=$( (command -v nproc >/dev/null 2>&1 && nproc) || true )
  if [ -n "${n:-}" ]; then
    printf '%s\n' "$n"
    return
  fi

  n=$( (command -v getconf >/dev/null 2>&1 && getconf _NPROCESSORS_ONLN) || true )
  if [ -n "${n:-}" ]; then
    printf '%s\n' "$n"
    return
  fi

  n=$( (command -v sysctl >/dev/null 2>&1 && sysctl -n hw.ncpu) || true )
  printf '%s\n' "${n:-1}"
}

hide_output() {
  set +x
  
  cleanup() {
    if [ -n "${PING_LOOP_PID:-}" ]; then
      kill "$PING_LOOP_PID" 2>/dev/null || true
      wait "$PING_LOOP_PID" 2>/dev/null || true
      PING_LOOP_PID=""
    fi
    rm -f /tmp/build.log
  }

  on_err() {
    echo "ERROR: An error was encountered with the build."
    cat /tmp/build.log
    cleanup
    exit 1
  }
  
  trap on_err ERR INT TERM
  bash -c 'while true; do sleep 30; echo $(date) - building ...; done' &
  PING_LOOP_PID=$!
  "$@" &> /tmp/build.log
  trap - ERR INT TERM
  cleanup
  set -x
}

ARCH=$1
TARGET=$ARCH-linux-musl

# Don't depend on the mirrors of sabotage linux that musl-cross-make uses.
LINUX_HEADERS_SITE=https://ci-mirrors.rust-lang.org/rustc/sabotage-linux-tarballs
LINUX_VER=headers-4.19.88

# Use environment variable if set, otherwise default to /usr/local
OUTPUT=${OUTPUT:-/usr/local}
shift

# Ancient binutils versions don't understand debug symbols produced by more recent tools.
# Apparently applying `-fPIC` everywhere allows them to link successfully.
# Enable debug info. If we don't do so, users can't debug into musl code,
# debuggers can't walk the stack, etc. Fixes #90103.
export CFLAGS="-fPIC -g1 $CFLAGS"

# Disable NLS (gettext/libintl). It's not required for a working musl toolchain,
# and on macOS it can pick up host libintl headers (e.g. /usr/local/include),
# causing build failures (e.g. `setlocale` macro conflicts).
#
# Note: musl-cross-make's Makefiles set COMMON_CONFIG internally, so exporting it
# via the environment is not enough. We pass it as a make variable below.
COMMON_CONFIG_FOR_BUILD="${COMMON_CONFIG:-}"
if [[ "${COMMON_CONFIG_FOR_BUILD}" != *"--disable-nls"* ]]; then
  COMMON_CONFIG_FOR_BUILD="${COMMON_CONFIG_FOR_BUILD} --disable-nls"
fi

# Create temporary directory for build
WORKDIR="${WORKDIR:-.}"
mkdir -p "$WORKDIR"
cd "$WORKDIR"

git clone https://github.com/richfelker/musl-cross-make # -b v0.9.9
cd musl-cross-make
# A version that includes support for building musl 1.2.3
git checkout fe915821b652a7fa37b34a596f47d8e20bc72338

if [ "$(uname -s)" = "Darwin" ]; then
  # Generate patches to fix issues on modern macOS
  BINUTILS_DIR=patches/binutils-2.33.1
  GCC_DIR=patches/gcc-9.4.0

  mkdir -p "${BINUTILS_DIR}"
  mkdir -p "${GCC_DIR}"

  # Fix zlib's fdopen detection on macOS
  # Problem: zlib incorrectly detects macOS and disables fdopen, causing build failures
  # Solution: Only apply the old MACOS behavior for non-Apple compilers
  cat <<'PATCH' > "${BINUTILS_DIR}/9001-fix-zlib-fdopen-macos.patch"
--- binutils-2.33.1.orig/zlib/zutil.h   2019-09-09 21:19:45
+++ binutils-2.33.1/zlib/zutil.h        2025-11-13 23:06:51
@@ -130,7 +130,7 @@
 #  endif
 #endif
 
-#if defined(MACOS) || defined(TARGET_OS_MAC)
+#if defined(MACOS) || (defined(TARGET_OS_MAC) && !defined(__APPLE__))
 #  define OS_CODE  7
 #  ifndef Z_SOLO
 #    if defined(__MWERKS__) && __dest_os != __be_os && __dest_os != __win32_os
PATCH

  # Fix zlib's fdopen detection on macOS
  # Problem: zlib incorrectly detects macOS and disables fdopen, causing build failures
  # Solution: Only apply the old MACOS behavior for non-Apple compilers
  cat <<'PATCH' > "${GCC_DIR}/9005-fix-zlib-fdopen-macos.patch"
--- gcc-9.4.0.orig/zlib/zutil.h   2019-09-09 21:19:45
+++ gcc-9.4.0/zlib/zutil.h        2025-11-13 23:06:51
@@ -130,7 +130,7 @@
 #  endif
 #endif
 
-#if defined(MACOS) || defined(TARGET_OS_MAC)
+#if defined(MACOS) || (defined(TARGET_OS_MAC) && !defined(__APPLE__))
 #  define OS_CODE  7
 #  ifndef Z_SOLO
 #    if defined(__MWERKS__) && __dest_os != __be_os && __dest_os != __win32_os
PATCH

  # Fix libctf inline functions causing linker errors on macOS
  # Problem: 'inline' without 'static' in header files causes undefined symbol errors
  #          on macOS when linking (e.g., "_bswap_16" not found)
  # Solution: Change 'inline' to 'static inline' to ensure each translation unit gets its own copy
  cat <<'PATCH' > "${BINUTILS_DIR}/9002-fix-libctf-swap-macos.patch"
--- binutils-2.33.1.orig/libctf/swap.h  2025-11-14 11:46:00
+++ binutils-2.33.1/libctf/swap.h      2025-11-14 11:45:23
@@ -28,13 +28,13 @@
 #else
 
 /* Provide our own versions of the byteswap functions.  */
-inline uint16_t
+static inline uint16_t
 bswap_16 (uint16_t v)
 {
   return ((v >> 8) & 0xff) | ((v & 0xff) << 8);
 }
 
-inline uint32_t
+static inline uint32_t
 bswap_32 (uint32_t v)
 {
   return (  ((v & 0xff000000) >> 24)
@@ -43,13 +43,13 @@
          | ((v & 0x000000ff) << 24));
 }
 
-inline uint64_t
+static inline uint64_t
 bswap_identity_64 (uint64_t v)
 {
   return v;
 }
 
-inline uint64_t
+static inline uint64_t
 bswap_64 (uint64_t v)
 {
   return (  ((v & 0xff00000000000000ULL) >> 56)
PATCH

  # Fix conflicting getcwd() declaration in binutils intl
  # Problem: Old K&R-style 'char *getcwd();' declaration conflicts with modern
  #          POSIX 'char *getcwd(char *, size_t)' from unistd.h on macOS SDK
  # Solution: Only declare getcwd() if unistd.h is NOT included
  cat <<'PATCH' > "${BINUTILS_DIR}/9003-fix-intl-dcigettext-macos.patch"
--- binutils-2.33.1.orig/intl/dcigettext.c      2019-09-09 21:19:44
+++ binutils-2.33.1/intl/dcigettext.c  2025-11-14 15:49:56
@@ -147,7 +147,7 @@
 # if !defined HAVE_GETCWD
 char *getwd ();
 #  define getcwd(buf, max) getwd (buf)
-# else
+# elif !defined HAVE_UNISTD_H 
 char *getcwd ();
 # endif
 # ifndef HAVE_STPCPY
PATCH

  # Fix conflicting getcwd() declaration in GCC intl (same issue as binutils)
  # Problem: Old K&R-style 'char *getcwd();' declaration conflicts with modern
  #          POSIX 'char *getcwd(char *, size_t)' from unistd.h on macOS SDK
  # Solution: Only declare getcwd() if unistd.h is NOT included
  cat <<'PATCH' > "${GCC_DIR}/9004-fix-intl-dcigettext-macos.patch"
--- gcc-9.4.0.orig/intl/dcigettext.c      2019-09-09 21:19:44
+++ gcc-9.4.0/intl/dcigettext.c  2025-11-14 15:49:56
@@ -147,7 +147,7 @@
 # if !defined HAVE_GETCWD
 char *getwd ();
 #  define getcwd(buf, max) getwd (buf)
-# else
+# elif !defined HAVE_UNISTD_H 
 char *getcwd ();
 # endif
 # ifndef HAVE_STPCPY
PATCH

  # Add ARM64 macOS host-specific hooks to GCC (ARM64-specific)
  # Problem: GCC 9.4.0 lacks host_hooks definition for ARM64 macOS, causing
  #          "_host_hooks" undefined symbol errors during linking
  # Solution: Create host-aarch64-darwin.c and x-darwin build file, register in config.host
  #           This follows the same pattern as i386/x86_64 Darwin hosts
  cat <<'PATCH' > "${GCC_DIR}/9006-fix-aarch64-darwin-host.patch"
--- /dev/null
+++ gcc-9.4.0/gcc/config/aarch64/host-aarch64-darwin.c
@@ -0,0 +1,32 @@
+/* aarch64-darwin host-specific hook definitions.
+   Copyright (C) 2003-2019 Free Software Foundation, Inc.
+
+This file is part of GCC.
+
+GCC is free software; you can redistribute it and/or modify it under
+the terms of the GNU General Public License as published by the Free
+Software Foundation; either version 3, or (at your option) any later
+version.
+
+GCC is distributed in the hope that it will be useful, but WITHOUT ANY
+WARRANTY; without even the implied warranty of MERCHANTABILITY or
+FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
+for more details.
+
+You should have received a copy of the GNU General Public License
+along with GCC; see the file COPYING3.  If not see
+<http://www.gnu.org/licenses/>.  */
+
+#define IN_TARGET_CODE 1
+
+#include "config.h"
+#include "system.h"
+#include "coretypes.h"
+#include "hosthooks.h"
+#include "hosthooks-def.h"
+#include "config/host-darwin.h"
+
+/* Darwin doesn't do anything special for aarch64 hosts; this file exists just
+   to include config/host-darwin.h.  */
+
+const struct host_hooks host_hooks = HOST_HOOKS_INITIALIZER;
--- /dev/null
+++ gcc-9.4.0/gcc/config/aarch64/x-darwin
@@ -0,0 +1,3 @@
+host-aarch64-darwin.o : $(srcdir)/config/aarch64/host-aarch64-darwin.c
+	$(COMPILE) $<
+	$(POSTCOMPILE)
--- gcc-9.4.0.orig/gcc/config.host
+++ gcc-9.4.0/gcc/config.host
@@ -254,6 +254,10 @@ case ${host} in
   i[34567]86-*-darwin* | x86_64-*-darwin*)
     out_host_hook_obj="${out_host_hook_obj} host-i386-darwin.o"
     host_xmake_file="${host_xmake_file} i386/x-darwin"
+    ;;
+  aarch64-*-darwin*)
+    out_host_hook_obj="${out_host_hook_obj} host-aarch64-darwin.o"
+    host_xmake_file="${host_xmake_file} aarch64/x-darwin"
     ;;
   powerpc-*-darwin*)
     out_host_hook_obj="${out_host_hook_obj} host-ppc-darwin.o"
PATCH

  # Fix ARM64 detection in GCC config.guess (ARM64-specific, same as binutils)
  # Problem: On Apple Silicon Macs, 'uname -p' returns 'arm' (32-bit identifier)
  #          instead of 'aarch64', causing incorrect host triplet 'arm-apple-darwin'
  # Solution: Map 'arm' and 'arm64' processor types to 'aarch64' for GNU toolchain compatibility
  cat <<'PATCH' > "${GCC_DIR}/9007-fix-config-guess-aarch64-darwin.patch"
--- gcc-9.4.0.orig/config.guess
+++ gcc-9.4.0/config.guess
@@ -1342,6 +1342,8 @@
 	    # processor. This is not true of the ARM version of Darwin
 	    # that Apple uses in portable devices.
 	    UNAME_PROCESSOR=x86_64
+	elif test "$UNAME_PROCESSOR" = arm -o "$UNAME_PROCESSOR" = arm64 ; then
+	    UNAME_PROCESSOR=aarch64
 	fi
 	echo "$UNAME_PROCESSOR"-apple-darwin"$UNAME_RELEASE"
 	exit ;;
PATCH

  cat <<'PATCH' > "${GCC_DIR}/9008-fix-macos-libcpp-ctype-conflict.patch"
--- gcc-9.4.0.orig/gcc/system.h	2025-11-16 07:14:02
+++ gcc-9.4.0/gcc/system.h	2025-11-16 07:15:22
@@ -201,19 +201,6 @@
 #ifdef INCLUDE_STRING
 # include <string>
 #endif
-#endif
-
-/* There are an extraordinary number of issues with <ctype.h>.
-   The last straw is that it varies with the locale.  Use libiberty's
-   replacement instead.  */
-#include "safe-ctype.h"
-
-#include <sys/types.h>
-
-#include <errno.h>
-
-#if !defined (errno) && defined (HAVE_DECL_ERRNO) && !HAVE_DECL_ERRNO
-extern int errno;
 #endif
 
 #ifdef __cplusplus
@@ -237,6 +224,19 @@
 # include <utility>
 #endif
 
+/* There are an extraordinary number of issues with <ctype.h>.
+   The last straw is that it varies with the locale.  Use libiberty's
+   replacement instead.  */
+   #include "safe-ctype.h"
+
+   #include <sys/types.h>
+   
+   #include <errno.h>
+   
+   #if !defined (errno) && defined (HAVE_DECL_ERRNO) && !HAVE_DECL_ERRNO
+   extern int errno;
+   #endif
+
 /* Some of glibc's string inlines cause warnings.  Plus we'd rather
    rely on (and therefore test) GCC's string builtins.  */
 #define __NO_STRING_INLINES
PATCH
fi

hide_output make -j"$(cpu_count)" TARGET=$TARGET MUSL_VER=1.2.3 LINUX_HEADERS_SITE=$LINUX_HEADERS_SITE LINUX_VER=$LINUX_VER GCC_CONFIG_FOR_TARGET="$GCC_CONFIG_FOR_TARGET" COMMON_CONFIG="$COMMON_CONFIG_FOR_BUILD"
hide_output make install TARGET=$TARGET MUSL_VER=1.2.3 LINUX_HEADERS_SITE=$LINUX_HEADERS_SITE LINUX_VER=$LINUX_VER OUTPUT=$OUTPUT GCC_CONFIG_FOR_TARGET="$GCC_CONFIG_FOR_TARGET" COMMON_CONFIG="$COMMON_CONFIG_FOR_BUILD"

printf '!<arch>\n' | tee $OUTPUT/$TARGET/lib/libunwind.a > /dev/null

