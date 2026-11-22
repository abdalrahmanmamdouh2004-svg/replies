# Project shell for the Watch Menu app (the zip had only app/)
1. Copy the patched `app` folder into this folder (next to settings.gradle.kts).
2. echo "sdk.dir=$ANDROID_HOME" > local.properties
3. ./gradlew :app:compileDebugKotlin     (check)   then   ./gradlew :app:assembleDebug
