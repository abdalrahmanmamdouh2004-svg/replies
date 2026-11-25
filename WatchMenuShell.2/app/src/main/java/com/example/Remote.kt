package com.example

import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothHidDevice
import android.bluetooth.BluetoothHidDeviceAppSdpSettings
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import java.util.concurrent.Executors
import kotlinx.coroutines.flow.MutableStateFlow

/**
 * Serves the separate watch app "Watch Remote" (messages start with "c1", see its main.rs). The phone acts as a
 * Bluetooth keyboard + mouse (HID) for a PC. Watch presses are mapped to actions here, so shortcuts and the mouse
 * sensitivity are changed on the phone. Presses only act while the watch app is open (heartbeat), and the phone is
 * an ordinary paired phone otherwise. The gyroscope mouse is NOT implemented yet: mouse mode moves with the arrows.
 */
@SuppressLint("MissingPermission")
object Remote {
    enum class Act(val label: String) {
        NONE("Nothing"), UP("Up"), DOWN("Down"), LEFT("Left"), RIGHT("Right"), ENTER("Enter"), ESC("Esc"), TAB("Tab"),
        SPACE("Space"), BACKSPACE("Backspace"), LEFT_CLICK("Left click"), RIGHT_CLICK("Right click"),
        MIDDLE_CLICK("Middle click"), DOUBLE_CLICK("Double click"), MOUSE_MODE("Mouse mode on/off"),
        SCROLL_UP("Scroll up"), SCROLL_DOWN("Scroll down"),
    }

    val BUTTONS = listOf(1 to "Up", 2 to "Down", 3 to "Left", 4 to "Right", 5 to "Centre", 6 to "Mode (small)")
    val GESTURES = listOf(0 to "tap", 1 to "long press", 2 to "double tap", 3 to "triple tap")
    /** Direction buttons send only tap and long press; centre and mode send all four. */
    fun gesturesOf(button: Int) = if (button <= 4) GESTURES.take(2) else GESTURES

    private const val EXPIRE_MS = 9_000L
    private const val PREFS = "remote"

    val status = MutableStateFlow("Off")
    val hostName = MutableStateFlow<String?>(null)
    val watchOpen = MutableStateFlow(false)
    val mouseMode = MutableStateFlow(false)
    val enabledFlow = MutableStateFlow(false)

    private val handler = Handler(Looper.getMainLooper())
    private val executor = Executors.newSingleThreadExecutor()
    private var ctx: Context? = null
    private var hid: BluetoothHidDevice? = null
    private var host: BluetoothDevice? = null
    private var registered = false
    private var slot = -1
    private var lastBeat = 0L

    // ---- settings ----

    private fun prefs(c: Context) = c.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    private fun context(): Context? = ctx ?: MediaListenerService.instance?.applicationContext
    fun enabled(c: Context) = prefs(c).getBoolean("enabled", false)
    fun sensitivity(c: Context) = prefs(c).getInt("sens", 20).coerceIn(1, 100)
    fun setSensitivity(c: Context, v: Int) = prefs(c).edit().putInt("sens", v.coerceIn(1, 100)).apply()
    fun lastHost(c: Context) = prefs(c).getString("host", null)

    fun default(button: Int, gesture: Int): Act = when (button) {
        1 -> Act.UP
        2 -> Act.DOWN
        3 -> Act.LEFT
        4 -> Act.RIGHT
        5 -> Act.RIGHT_CLICK
        else -> Act.NONE
    }

    fun action(c: Context, button: Int, gesture: Int): Act {
        val name = prefs(c).getString("n_${button}_$gesture", null) ?: return default(button, gesture)
        return Act.values().firstOrNull { it.name == name } ?: default(button, gesture)
    }

    fun setAction(c: Context, button: Int, gesture: Int, a: Act) =
        prefs(c).edit().putString("n_${button}_$gesture", a.name).apply()

    fun resetActions(c: Context) {
        val e = prefs(c).edit()
        prefs(c).all.keys.filter { it.startsWith("n_") }.forEach { e.remove(it) }
        e.apply()
    }

    // ---- Bluetooth HID ----

    private val descriptor = byteArrayOf(
        // keyboard, report id 1
        0x05, 0x01, 0x09, 0x06, 0xA1.toByte(), 0x01, 0x85.toByte(), 0x01, 0x05, 0x07, 0x19, 0xE0.toByte(), 0x29, 0xE7.toByte(),
        0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95.toByte(), 0x08, 0x81.toByte(), 0x02, 0x95.toByte(), 0x01, 0x75, 0x08,
        0x81.toByte(), 0x01, 0x95.toByte(), 0x05, 0x75, 0x01, 0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91.toByte(), 0x02,
        0x95.toByte(), 0x01, 0x75, 0x03, 0x91.toByte(), 0x01, 0x95.toByte(), 0x06, 0x75, 0x08, 0x15, 0x00, 0x25, 0x65,
        0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x81.toByte(), 0x00, 0xC0.toByte(),
        // mouse, report id 2: 3 buttons, x, y, wheel
        0x05, 0x01, 0x09, 0x02, 0xA1.toByte(), 0x01, 0x85.toByte(), 0x02, 0x09, 0x01, 0xA1.toByte(), 0x00, 0x05, 0x09,
        0x19, 0x01, 0x29, 0x03, 0x15, 0x00, 0x25, 0x01, 0x95.toByte(), 0x03, 0x75, 0x01, 0x81.toByte(), 0x02,
        0x95.toByte(), 0x01, 0x75, 0x05, 0x81.toByte(), 0x01, 0x05, 0x01, 0x09, 0x30, 0x09, 0x31, 0x09, 0x38,
        0x15, 0x81.toByte(), 0x25, 0x7F, 0x75, 0x08, 0x95.toByte(), 0x03, 0x81.toByte(), 0x06, 0xC0.toByte(), 0xC0.toByte(),
    )

    private val callback = object : BluetoothHidDevice.Callback() {
        override fun onAppStatusChanged(pluggedDevice: BluetoothDevice?, registeredNow: Boolean) {
            registered = registeredNow
            handler.post {
                status.value = if (registeredNow) "Ready: pair this phone from the PC's Bluetooth settings" else "Not registered"
                if (registeredNow) context()?.let { c -> lastHost(c)?.let { addr -> connectTo(c, addr) } }
            }
        }

        override fun onConnectionStateChanged(device: BluetoothDevice?, state: Int) {
            handler.post {
                when (state) {
                    BluetoothProfile.STATE_CONNECTED -> {
                        host = device
                        hostName.value = device?.name ?: device?.address
                        status.value = "Connected"
                        context()?.let { prefs(it).edit().putString("host", device?.address).apply() }
                    }
                    BluetoothProfile.STATE_DISCONNECTED -> {
                        if (host?.address == device?.address) { host = null; hostName.value = null }
                        if (registered) status.value = "Not connected to a PC"
                    }
                }
            }
        }
    }

    fun setEnabled(c: Context, on: Boolean) {
        ctx = c.applicationContext
        prefs(c).edit().putBoolean("enabled", on).apply()
        enabledFlow.value = on
        if (on) start(c) else stop()
    }

    /** Starts the HID service if it is enabled (safe to call repeatedly). */
    fun start(c: Context) {
        ctx = c.applicationContext
        enabledFlow.value = enabled(c)
        if (!enabledFlow.value || hid != null) return
        if (android.os.Build.VERSION.SDK_INT < 28) { status.value = "Needs Android 9 or newer"; return }
        val adapter = (c.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager)?.adapter
        if (adapter == null || !adapter.isEnabled) { status.value = "Bluetooth is off"; return }
        try {
            status.value = "Starting…"
            adapter.getProfileProxy(c.applicationContext, object : BluetoothProfile.ServiceListener {
                override fun onServiceConnected(profile: Int, proxy: BluetoothProfile) {
                    val h = proxy as BluetoothHidDevice
                    hid = h
                    val sdp = BluetoothHidDeviceAppSdpSettings(
                        "Watch Menu Remote", "Watch remote", "Watch Menu", BluetoothHidDevice.SUBCLASS1_COMBO, descriptor
                    )
                    val ok = h.registerApp(sdp, null, null, executor, callback)
                    if (!ok) status.value = "Could not register as a keyboard/mouse (this phone may not support it)"
                }

                override fun onServiceDisconnected(profile: Int) {
                    hid = null; registered = false; host = null; hostName.value = null
                    status.value = "Stopped"
                }
            }, BluetoothProfile.HID_DEVICE)
        } catch (e: SecurityException) {
            status.value = "Bluetooth permission needed"
        } catch (e: Exception) {
            AppLogger.logError("Remote", "start", e); status.value = "Error: ${e.message}"
        }
    }

    fun stop() {
        try { hid?.unregisterApp() } catch (_: Exception) {}
        try {
            val adapter = BluetoothAdapter.getDefaultAdapter()
            hid?.let { adapter?.closeProfileProxy(BluetoothProfile.HID_DEVICE, it) }
        } catch (_: Exception) {}
        hid = null; registered = false; host = null; hostName.value = null
        status.value = "Off"
    }

    fun bonded(): List<BluetoothDevice> = try {
        BluetoothAdapter.getDefaultAdapter()?.bondedDevices?.toList().orEmpty()
    } catch (_: SecurityException) { emptyList() }

    fun connectTo(c: Context, address: String) {
        val d = bonded().firstOrNull { it.address == address } ?: return
        try { hid?.connect(d) } catch (e: Exception) { AppLogger.logError("Remote", "connect", e) }
    }

    fun disconnect() { try { host?.let { hid?.disconnect(it) } } catch (_: Exception) {} }

    // ---- input to the PC ----

    private fun send(id: Int, data: ByteArray) {
        val h = hid ?: return
        val d = host ?: return
        try { h.sendReport(d, id, data) } catch (e: Exception) { AppLogger.logError("Remote", "send", e) }
    }

    private fun keyCode(a: Act) = when (a) {
        Act.RIGHT -> 0x4F; Act.LEFT -> 0x50; Act.DOWN -> 0x51; Act.UP -> 0x52
        Act.ENTER -> 0x28; Act.ESC -> 0x29; Act.BACKSPACE -> 0x2A; Act.TAB -> 0x2B; Act.SPACE -> 0x2C
        else -> 0
    }

    private fun key(code: Int) {
        send(1, byteArrayOf(0, 0, code.toByte(), 0, 0, 0, 0, 0))
        send(1, ByteArray(8))
    }

    private fun mouse(buttons: Int, dx: Int, dy: Int, wheel: Int = 0) =
        send(2, byteArrayOf(buttons.toByte(), dx.coerceIn(-127, 127).toByte(), dy.coerceIn(-127, 127).toByte(), wheel.coerceIn(-127, 127).toByte()))

    private fun click(mask: Int) { mouse(mask, 0, 0); mouse(0, 0, 0) }

    /** Moves the cursor [dist] units (several reports when it is more than one report can carry). */
    fun move(dx: Int, dy: Int) {
        var x = dx; var y = dy
        while (x != 0 || y != 0) {
            val sx = x.coerceIn(-127, 127); val sy = y.coerceIn(-127, 127)
            mouse(0, sx, sy); x -= sx; y -= sy
        }
    }

    fun perform(c: Context, a: Act, long: Boolean) {
        val step = sensitivity(c) * (if (long) 5 else 1)
        when (a) {
            Act.NONE -> {}
            Act.UP, Act.DOWN, Act.LEFT, Act.RIGHT ->
                key(keyCode(a))
            Act.ENTER, Act.ESC, Act.TAB, Act.SPACE, Act.BACKSPACE -> key(keyCode(a))
            Act.LEFT_CLICK -> click(1)
            Act.RIGHT_CLICK -> click(2)
            Act.MIDDLE_CLICK -> click(4)
            Act.DOUBLE_CLICK -> { click(1); click(1) }
            Act.MOUSE_MODE -> {}
            Act.SCROLL_UP -> mouse(0, 0, 0, 3)
            Act.SCROLL_DOWN -> mouse(0, 0, 0, -3)
        }
    }

    /** A press (from the watch, or from the pad on the phone's own screen). */
    fun press(c: Context, button: Int, gesture: Int) =
        perform(c, action(c, button, gesture), gesture == 1)

    // ---- the watch app ----

    private val expire = Runnable { if (SystemClock.elapsedRealtime() - lastBeat > EXPIRE_MS) closeApp() }

    private fun markOpen(s: Int) {
        slot = s
        lastBeat = SystemClock.elapsedRealtime()
        if (!watchOpen.value) {
            watchOpen.value = true
            context()?.let { start(it) }
        }
        handler.removeCallbacks(expire)
        handler.postDelayed(expire, EXPIRE_MS + 250L)
    }

    private fun closeApp() {
        handler.removeCallbacks(expire)
        slot = -1
        watchOpen.value = false
        mouseMode.value = false
    }

    /** A ping said which app slot runs. True if it is the Remote app (the others then ignore it). */
    fun onPing(p: ByteArray): Boolean {
        if (p.size < 3) return false
        val s = p[1].toInt() and 0xFF
        if (s != slot) {
            if (watchOpen.value && s != 0xFF && (p[2].toInt() and 1) != 0) handler.post { closeApp() }
            return false
        }
        if ((p[2].toInt() and 1) == 0) handler.post { closeApp() }
        return true
    }

    /** Another watch app said hello: the Remote app is not the one running. */
    fun otherAppHello() { if (watchOpen.value) handler.post { closeApp() } }

    fun onEvent(s: Int, d: ByteArray) {
        when (d[2].toInt().toChar()) {
            'h', 'f' -> handler.post { markOpen(s) }
            'z' -> handler.post { closeApp() }
            'k' -> if (d.size >= 5) {
                val b = d[3].toInt() and 0xFF
                val g = d[4].toInt() and 0xFF
                handler.post { markOpen(s); context()?.let { press(it, b, g) } }
            }
        }
    }
}
