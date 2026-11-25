package com.example

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.Bundle
import android.os.IBinder
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import java.util.Locale

/**
 * Phone-microphone dictation for Watch Replies.
 * The service is promoted while HomeActivity is visible, then stays ready in the background.
 * SpeechRecognizer uses the phone's normal microphone; no HFP call or watch-audio routing is involved.
 */
class PhoneDictationService : Service() {
    companion object {
        private const val CHANNEL_ID = "watch_replies_voice"
        private const val NOTIFICATION_ID = 7314
        private const val ACTION_READY = "com.example.watchreplies.voice.READY"
            @Volatile private var active: PhoneDictationService? = null

        fun ensureReady(context: Context) {
            val intent = Intent(context, PhoneDictationService::class.java).setAction(ACTION_READY)
            try {
                ContextCompat.startForegroundService(context, intent)
            } catch (e: Exception) {
                AppLogger.logError("VoiceDictation", "start foreground microphone service", e)
            }
        }

        /** The service is pre-started from HomeActivity; no background service launch is attempted here. */
        fun toggle(languageTag: String) {
            val service = active
            if (service == null) {
                AppLogger.log("VoiceDictation", "Service is not ready; reopen Watch Menu once on the phone")
                Replies.voiceStatus(2)
            } else {
                android.os.Handler(service.mainLooper).post { service.toggleFromWatch(languageTag) }
            }
        }
    }

    private var recognizer: SpeechRecognizer? = null
    private var listening = false
    private var stopRequested = false

    private val recognitionListener = object : RecognitionListener {
        override fun onReadyForSpeech(params: Bundle?) {
            updateNotification("Listening on the phone microphone…")
        }
        override fun onBeginningOfSpeech() = Unit
        override fun onRmsChanged(rmsdB: Float) = Unit
        override fun onBufferReceived(buffer: ByteArray?) = Unit
        override fun onEndOfSpeech() = Unit
        override fun onPartialResults(partialResults: Bundle?) = Unit
        override fun onEvent(eventType: Int, params: Bundle?) = Unit

        override fun onResults(results: Bundle?) {
            val spoken = results?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                ?.firstOrNull()?.trim().orEmpty()
            listening = false
            stopRequested = false
            updateNotification("Ready — tap the microphone on Watch Replies")
            if (spoken.isNotBlank()) {
                Replies.voiceStatus(0)
                Replies.voiceText(spoken)
            } else {
                Replies.voiceStatus(2)
            }
        }

        override fun onError(error: Int) {
            listening = false
            stopRequested = false
            updateNotification("Voice input unavailable — check speech services or try again")
            Replies.voiceStatus(2)
            android.os.Handler(mainLooper).postDelayed({
                if (!listening) updateNotification("Ready — tap the microphone on Watch Replies")
            }, 3500L)
        }
    }

    override fun onCreate() {
        super.onCreate()
        active = this
        createNotificationChannel()
        promoteToForeground("Ready — tap the microphone on Watch Replies")
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // Toggle requests are delivered directly to the live service instance by Replies.
        return START_NOT_STICKY
    }

    private fun toggleFromWatch(languageTag: String) {
        if (listening) requestStop() else startDictation(languageTag)
    }

    private fun startDictation(languageTag: String) {
        if (checkSelfPermission(android.Manifest.permission.RECORD_AUDIO) != android.content.pm.PackageManager.PERMISSION_GRANTED) {
            updateNotification("Open Watch Menu and allow microphone permission")
            Replies.voiceStatus(2)
            return
        }
        if (!SpeechRecognizer.isRecognitionAvailable(this)) {
            updateNotification("No speech recognition service is available")
            Replies.voiceStatus(2)
            return
        }
        try {
            val engine = recognizer ?: SpeechRecognizer.createSpeechRecognizer(this).also {
                it.setRecognitionListener(recognitionListener)
                recognizer = it
            }
            stopRequested = false
            listening = true
            updateNotification("Listening on the phone microphone…")
            Replies.voiceStatus(1)
            val request = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
                putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
                // The watch keyboard selects English or Egyptian Arabic; do not use the phone's global locale.
                putExtra(RecognizerIntent.EXTRA_LANGUAGE, languageTag)
                putExtra(RecognizerIntent.EXTRA_LANGUAGE_PREFERENCE, languageTag)
                putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 1)
                putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, false)
                putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, false)
            }
            engine.startListening(request)
        } catch (e: Exception) {
            listening = false
            stopRequested = false
            AppLogger.logError("VoiceDictation", "start speech recognition", e)
            updateNotification("Could not start speech recognition")
            Replies.voiceStatus(2)
        }
    }

    /** A second tap asks Android to finish the current phrase and return its recognized text. */
    private fun requestStop() {
        if (!listening || stopRequested) return
        stopRequested = true
        updateNotification("Finishing dictation…")
        try {
            recognizer?.stopListening()
        } catch (e: Exception) {
            AppLogger.logError("VoiceDictation", "stop speech recognition", e)
            listening = false
            stopRequested = false
            Replies.voiceStatus(2)
            updateNotification("Ready — tap the microphone on Watch Replies")
        }
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(CHANNEL_ID, "Watch Replies voice input", NotificationManager.IMPORTANCE_LOW).apply {
                description = "Shows when Watch Replies is ready to dictate from the phone microphone"
                setSound(null, null)
                enableVibration(false)
            }
            getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
        }
    }

    private fun notification(message: String): Notification {
        val open = Intent(this, HomeActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        val pending = PendingIntent.getActivity(
            this, 7314, open,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_voice_mic)
            .setContentTitle("Watch Replies")
            .setContentText(message)
            .setContentIntent(pending)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .build()
    }

    private fun promoteToForeground(message: String) {
        val n = notification(message)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIFICATION_ID, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE)
        } else {
            startForeground(NOTIFICATION_ID, n)
        }
    }

    private fun updateNotification(message: String) {
        getSystemService(NotificationManager::class.java).notify(NOTIFICATION_ID, notification(message))
    }

    override fun onDestroy() {
        listening = false
        if (active === this) active = null
        try { recognizer?.cancel() } catch (_: Exception) { }
        recognizer?.destroy()
        recognizer = null
        stopForeground(STOP_FOREGROUND_REMOVE)
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
