package com.example

import android.app.Notification
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.service.notification.StatusBarNotification
import android.app.RemoteInput
import java.text.Normalizer
import java.util.Calendar

/**
 * Serves the separate watch app "Watch Replies": the chats of WhatsApp, Messenger and Telegram that can be answered
 * from their notification, shown on the watch and answered with the watch's keyboard.
 *
 * Messages all start with "r1" (see the watch app's proto.rs), so they never mix with Watch Music's messages. Nothing
 * here does anything until the Replies app says hello, so the phone works the same with only one of the two watch apps
 * installed, or both (only one runs on the watch at a time).
 */
object Replies {
    private const val MSG = 0x90A5
    private const val MAX_CHATS = 8
    private const val NAME_MAX = 24
    private const val TEXT_MAX = 96
    private val QUICK = listOf("Ok", "On my way", "Thanks!")

    private class Chat(
        val id: Int, val key: String, val app: Int, var name: String, var text: String, var at: Long,
        var intent: PendingIntent, var inputs: Array<RemoteInput>,
        var msgs: List<Pair<String, Long>> = emptyList(), var fresh: List<Pair<String, Long>> = emptyList(),
    )

    private val handler = Handler(Looper.getMainLooper())
    private val chats = LinkedHashMap<String, Chat>() // oldest first
    private var open = false
    private var slot = -1
    private var app: Context? = null

    // ---- notifications (called from MediaListenerService) ----

    private fun appOf(pkg: String) = when (pkg) {
        "com.whatsapp", "com.whatsapp.w4b" -> 1
        "com.facebook.orca", "com.facebook.mlite" -> 2
        "org.telegram.messenger", "org.telegram.messenger.web", "org.telegram.plus" -> 3
        else -> 0
    }

    fun onPosted(c: Context, sbn: StatusBarNotification) {
        handler.post { loadOne(c, sbn)?.let { ch -> if (open) ch.fresh.forEach { enqueue(listMessage(ch, it.first, it.second)) } } }
    }

    /** Takes the notification in as a chat (on the main thread); null if it is not an answerable chat message. */
    private fun loadOne(c: Context, sbn: StatusBarNotification): Chat? {
        val kind = appOf(sbn.packageName)
        if (kind == 0) return null
        val n = sbn.notification ?: return null
        if (n.flags and Notification.FLAG_GROUP_SUMMARY != 0) return null
        val action = n.actions?.firstOrNull { !it.remoteInputs.isNullOrEmpty() && it.actionIntent != null } ?: return null
        val ex = n.extras ?: return null
        var title = (ex.getCharSequence(Notification.EXTRA_CONVERSATION_TITLE) ?: ex.getCharSequence(Notification.EXTRA_TITLE))?.toString().orEmpty()
        var body = ex.getCharSequence(Notification.EXTRA_TEXT)?.toString().orEmpty()
        val msgs = ex.getParcelableArray(Notification.EXTRA_MESSAGES)
        val all = ArrayList<Pair<String, Long>>()
        val isGroup = ex.getBoolean(Notification.EXTRA_IS_GROUP_CONVERSATION, false) || ex.getCharSequence(Notification.EXTRA_CONVERSATION_TITLE) != null
        msgs?.forEach { p ->
            val m = p as? Bundle ?: return@forEach
            val t = m.getCharSequence("text")?.toString()?.takeIf { it.isNotBlank() } ?: return@forEach
            val who = m.getCharSequence("sender")?.toString().orEmpty()
            val line = if (isGroup && who.isNotBlank() && who != title) "$who: $t" else t
            all.add(ascii(line, TEXT_MAX) to m.getLong("time", sbn.postTime))
        }
        (msgs?.lastOrNull() as? Bundle)?.let { m ->
            m.getCharSequence("text")?.toString()?.takeIf { it.isNotBlank() }?.let { body = it }
            if (title.isBlank()) title = m.getCharSequence("sender")?.toString().orEmpty()
        }
        if (title.isBlank() && body.isBlank()) return null
        val name = ascii(title, NAME_MAX)
        val text = ascii(body, TEXT_MAX)
        val inputs = action.remoteInputs ?: return null
        return run {
            app = c.applicationContext
            val old = chats.remove(sbn.key)
            val id = old?.id ?: freeId()
            val now = if (all.isEmpty()) listOf(text to sbn.postTime) else all.takeLast(6)
            // only what the watch has not been given yet: drop the part of the new list it already knows
            val known = old?.msgs.orEmpty().map { it.first }
            var k = minOf(known.size, now.size)
            while (k > 0 && known.takeLast(k) != now.take(k).map { it.first }) k--
            val fresh = if (old == null) now else now.drop(k).ifEmpty { if (known.lastOrNull() == now.last().first) emptyList() else listOf(now.last()) }
            val chat = Chat(id, sbn.key, kind, name, text, sbn.postTime, action.actionIntent, inputs, now, fresh)
            chats[sbn.key] = chat
            while (chats.size > MAX_CHATS) {
                val oldest = chats.values.first()
                chats.remove(oldest.key)
                if (open) enqueue(byteArrayOf('r'.code.toByte(), '1'.code.toByte(), 'D'.code.toByte(), oldest.id.toByte()))
            }
            chat
        }
    }

    fun onRemoved(sbn: StatusBarNotification) {
        if (appOf(sbn.packageName) == 0) return
        handler.post {
            val gone = chats.remove(sbn.key) ?: return@post
            if (open) enqueue(byteArrayOf('r'.code.toByte(), '1'.code.toByte(), 'D'.code.toByte(), gone.id.toByte()))
        }
    }

    private fun freeId(): Int {
        val used = chats.values.map { it.id }.toSet()
        return (1..255).first { it !in used }
    }

    /** Plain ASCII of at most [max] bytes: accents dropped, other letters (and emoji) become one dot per run. */
    internal fun ascii(s: String, max: Int): String {
        val d = Normalizer.normalize(s, Normalizer.Form.NFD)
        val sb = StringBuilder()
        var dot = false
        for (ch in d) {
            val code = ch.code
            val mark = Character.getType(ch) == Character.NON_SPACING_MARK.toInt()
            when {
                mark -> {}
                code in 0x20..0x7E -> { sb.append(ch); dot = false }
                ch == '\n' || ch == '\t' -> { if (!sb.endsWith(" ")) sb.append(' '); dot = false }
                code < 0x20 -> {}
                else -> if (!dot && !Character.isLowSurrogate(ch)) { sb.append('·'); dot = true }
            }
        }
        val out = sb.toString().trim().replace(Regex(" {2,}"), " ")
        // the dot is two bytes in UTF-8; the watch draws it
        var bytes = 0
        val cut = StringBuilder()
        for (ch in out) {
            val w = if (ch == '·') 2 else 1
            if (bytes + w > max) break
            bytes += w; cut.append(ch)
        }
        return cut.toString()
    }

    // ---- frames from WatchApp ----

    /** A ping said which app slot runs. True if it is the Replies app (Watch Music then ignores it). */
    fun onPing(p: ByteArray): Boolean {
        if (p.size < 3) return false
        val s = p[1].toInt() and 0xFF
        if (s != slot) {
            // another app runs now: the Replies app is closed, and its acks are no longer ours
            if (open && s != 0xFF && (p[2].toInt() and 1) != 0) { open = false; clearQueue() }
            return false
        }
        if ((p[2].toInt() and 1) == 0) { open = false; clearQueue() }
        return true
    }

    /** Watch Music said hello from [s]: that slot is not the Replies app (an app can be moved). */
    fun musicHello(s: Int) {
        slot = if (s == slot) -1 else slot
        if (open) AppLogger.log("Replies", "Watch Music said hello: Replies closed")
        open = false
        clearQueue()
    }

    fun onEvent(s: Int, d: ByteArray) {
        AppLogger.log("Replies", "from watch: ${d.size} bytes, kind ${d[2].toInt().toChar()}")
        when (d[2].toInt().toChar()) {
            'h' -> handler.post { hello(s) }
            'x' -> if (d.size >= 5) { val copy = d.copyOf(); handler.post { reply(copy) } }
        }
    }

    // ---- sending: one message at a time, each acknowledged ----

    private val queue = ArrayDeque<ByteArray>()
    private var inFlight: ByteArray? = null
    private var tries = 0
    private val timeout = Runnable { onTimeout() }

    private fun enqueue(m: ByteArray) {
        queue.addLast(m)
        pump()
    }

    private fun pump() {
        if (inFlight != null || !open) return
        val m = queue.removeFirstOrNull() ?: return
        inFlight = m
        tries = 0
        transmit()
    }

    private fun transmit() {
        val m = inFlight ?: return
        AppLogger.log("Replies", "to watch: kind ${m[2].toInt().toChar()}, ${m.size} bytes, try $tries")
        WatchLink.appFrame(MSG, byteArrayOf(0xFF.toByte()) + m)
        handler.removeCallbacks(timeout)
        handler.postDelayed(timeout, 1500)
    }

    private fun onTimeout() {
        if (inFlight == null) return
        if (++tries > 3) clearQueue() else transmit()
    }

    private fun clearQueue() {
        handler.removeCallbacks(timeout)
        queue.clear()
        inFlight = null
    }

    /** An ack arrived; true if the Replies app is the one running (Watch Music then ignores it). */
    fun onAck(p: ByteArray): Boolean {
        if (!open || inFlight == null) return false // not ours: Watch Music's acks must pass
        handler.post {
            when (if (p.isNotEmpty()) p[0].toInt() and 0xFF else 0) {
                0 -> { handler.removeCallbacks(timeout); inFlight = null; pump() }
                8, 9 -> { handler.removeCallbacks(timeout); handler.postDelayed({ transmit() }, 80) } // inbox full / popup
                else -> clearQueue()
            }
        }
        return true
    }

    // ---- the watch app ----

    private fun hello(s: Int) {
        slot = s
        open = true
        AppLogger.log("Replies", "hello from the Replies app (slot $s), ${chats.size} chats")
        clearQueue()
        // chats whose notifications came before this process listened are still on the phone: take them in
        try { MediaListenerService.instance?.let { l -> l.activeNotifications?.sortedBy { it.postTime }?.forEach { loadOne(l.applicationContext, it) } } } catch (e: Exception) { AppLogger.logError("Replies", "active", e) }
        enqueue(byteArrayOf('r'.code.toByte(), '1'.code.toByte(), 'C'.code.toByte()))
        val bytes = ArrayList<Byte>()
        bytes.addAll(listOf('r'.code.toByte(), '1'.code.toByte(), 'Q'.code.toByte(), QUICK.size.toByte()))
        val mine = app?.let { c -> NotificationForwarder.quickReplies(c).map { ascii(it, 24) }.filter { it.isNotBlank() }.take(3) }.orEmpty().ifEmpty { QUICK }
        bytes[3] = mine.size.toByte()
        for (t in mine) { val b = t.toByteArray(); bytes.add(b.size.toByte()); bytes.addAll(b.toList()) }
        enqueue(bytes.toByteArray())
        for (c in chats.values) c.msgs.takeLast(4).forEach { enqueue(listMessage(c, it.first, it.second)) }
    }

    private fun listMessage(c: Chat, msg: String = c.text, at: Long = c.at): ByteArray {
        val cal = Calendar.getInstance().apply { timeInMillis = at }
        val name = c.name.toByteArray(Charsets.UTF_8)
        val text = msg.toByteArray(Charsets.UTF_8)
        return byteArrayOf('r'.code.toByte(), '1'.code.toByte(), 'L'.code.toByte(), c.id.toByte(), c.app.toByte(),
            cal.get(Calendar.HOUR_OF_DAY).toByte(), cal.get(Calendar.MINUTE).toByte(), name.size.toByte()) + name + text
    }

    private val pieces = HashMap<Int, ByteArray>() // chat id -> text so far
    private var nextPiece = HashMap<Int, Int>()

    /** d = "r1x" id flags text...: a reply from the watch, in pieces. */
    private fun reply(d: ByteArray) {
        val id = d[3].toInt() and 0xFF
        val flags = d[4].toInt() and 0xFF
        val n = flags and 15
        val last = flags and 0x80 != 0
        val part = d.copyOfRange(5, d.size)
        if (n == 0) { pieces[id] = part; nextPiece[id] = 1 }
        else if (nextPiece[id] == n) { pieces[id] = (pieces[id] ?: ByteArray(0)) + part; nextPiece[id] = n + 1 }
        else { pieces.remove(id); nextPiece.remove(id); result(id, false); return } // a piece went missing
        if (!last) return
        val text = String(pieces.remove(id) ?: return, Charsets.UTF_8).trim()
        nextPiece.remove(id)
        result(id, text.isNotEmpty() && fire(id, text))
    }

    private fun result(id: Int, ok: Boolean) {
        AppLogger.log("Replies", "reply to chat $id: ${if (ok) "sent" else "failed"}")
        queue.addFirst(byteArrayOf('r'.code.toByte(), '1'.code.toByte(), 'R'.code.toByte(), id.toByte(), if (ok) 0 else 1))
        pump()
    }

    private fun fire(id: Int, text: String): Boolean {
        val c = chats.values.firstOrNull { it.id == id } ?: return false
        val ctx = app ?: return false
        return try {
            val intent = Intent()
            val results = Bundle()
            c.inputs.forEach { results.putCharSequence(it.resultKey, text) }
            RemoteInput.addResultsToIntent(c.inputs, intent, results)
            c.intent.send(ctx, 0, intent)
            AppLogger.log("Replies", "Replied to ${c.name} (${c.key})")
            true
        } catch (e: Exception) {
            AppLogger.logError("Replies", "reply", e)
            false
        }
    }
}
