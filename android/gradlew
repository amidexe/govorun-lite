#!/bin/sh

APP_HOME="$(cd "$(dirname "$0")" && pwd -P)"
CLASSPATH="$APP_HOME/gradle/wrapper/gradle-wrapper.jar"

JAVACMD="java"
if [ -n "$JAVA_HOME" ] ; then
    JAVACMD="$JAVA_HOME/bin/java"
elif [ -x /opt/homebrew/opt/openjdk@17/bin/java ]; then
    # Homebrew on Apple Silicon does not register formula JDKs with java_home.
    JAVA_HOME=/opt/homebrew/opt/openjdk@17
    export JAVA_HOME
    JAVACMD="$JAVA_HOME/bin/java"
elif [ -x /usr/local/opt/openjdk@17/bin/java ]; then
    # Homebrew on Intel Macs.
    JAVA_HOME=/usr/local/opt/openjdk@17
    export JAVA_HOME
    JAVACMD="$JAVA_HOME/bin/java"
fi

exec "$JAVACMD" -Xmx64m -Xms64m $JAVA_OPTS \
    -classpath "$CLASSPATH" \
    org.gradle.wrapper.GradleWrapperMain "$@"
