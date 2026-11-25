package com.example

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp

/** The phone side of Watch Remote: on/off, which PC, the pad (also tappable here), sensitivity and shortcuts. */
@Composable
fun WatchRemoteScreen(onBack: () -> Unit) {
    val context = LocalContext.current
    val status by Remote.status.collectAsState()
    val host by Remote.hostName.collectAsState()
    val open by Remote.watchOpen.collectAsState()
    val mouse by Remote.mouseMode.collectAsState()
    val on by Remote.enabledFlow.collectAsState()
    var sens by remember { mutableStateOf(Remote.sensitivity(context).toFloat()) }
    var tick by remember { mutableStateOf(0) } // forces a redraw after a shortcut changes
    var picking by remember { mutableStateOf(false) }

    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { ok ->
        if (ok) Remote.setEnabled(context, true)
    }
    fun turnOn() {
        if (Build.VERSION.SDK_INT >= 31 &&
            context.checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) != PackageManager.PERMISSION_GRANTED
        ) permission.launch(Manifest.permission.BLUETOOTH_CONNECT) else Remote.setEnabled(context, true)
    }
    LaunchedEffectOnce { Remote.start(context) }
    BackHandler(enabled = picking) { picking = false }

    Column(Modifier.fillMaxSize()) {
        NTopBar(title = "Watch Remote", onBack = { if (picking) picking = false else onBack() })
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp)) {
            SGroup(
                { SSwitchRow("Bluetooth keyboard/mouse", on) { if (it) turnOn() else Remote.setEnabled(context, false) } },
                { SRow("Status", null, value = status) },
                { SRow("PC", null, value = host ?: "none", onClick = if (on) ({ picking = !picking }) else null, chevron = false) },
                { SRow("Watch app", null, value = if (open) (if (mouse) "open, mouse mode" else "open") else "closed") },
            )
            if (picking) {
                val devices = remember(picking) { Remote.bonded() }
                Text("Pair the phone from the PC's Bluetooth settings first, then pick the PC here:", color = NColors.Dim, modifier = Modifier.padding(12.dp))
                val deviceRows: List<@Composable () -> Unit> = if (devices.isEmpty()) {
                    listOf({ SRow("No paired devices", null) })
                } else {
                    devices.map { d ->
                        @Composable { SRow(d.name ?: d.address, null, onClick = { Remote.connectTo(context, d.address); picking = false }) }
                    }
                }
                SGroup(*deviceRows.toTypedArray())
            }

            Spacer(Modifier.height(16.dp))
            Text("Pad (works here too, for testing)", color = NColors.Dim, modifier = Modifier.padding(start = 8.dp))
            Column(Modifier.fillMaxWidth().padding(8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Button(onClick = { Remote.press(context, 1, 0) }, modifier = Modifier.size(72.dp)) { Text("^") }
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Button(onClick = { Remote.press(context, 3, 0) }, modifier = Modifier.size(72.dp)) { Text("<") }
                    Button(onClick = { Remote.press(context, 5, 0) }, modifier = Modifier.size(88.dp)) { Text("OK") }
                    Button(onClick = { Remote.press(context, 4, 0) }, modifier = Modifier.size(72.dp)) { Text(">") }
                }
                Button(onClick = { Remote.press(context, 2, 0) }, modifier = Modifier.size(72.dp)) { Text("v") }
                Button(onClick = { Remote.press(context, 6, 0) }) { Text(if (mouse) "Mode: mouse" else "Mode: keys") }
            }

            Spacer(Modifier.height(8.dp))
            Text("Mouse sensitivity (calibration): ${sens.toInt()} px per step, long press = 5 steps", color = NColors.Text, modifier = Modifier.padding(start = 8.dp))
            Slider(value = sens, onValueChange = { sens = it }, onValueChangeFinished = { Remote.setSensitivity(context, sens.toInt()) }, valueRange = 1f..100f)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = { Remote.setSensitivity(context, sens.toInt()); Remote.move(-sens.toInt(), 0) }) { Text("Test left") }
                Button(onClick = { Remote.setSensitivity(context, sens.toInt()); Remote.move(sens.toInt(), 0) }) { Text("Test right") }
            }

            Spacer(Modifier.height(16.dp))
            Text("Shortcuts (tap a row to change it)", color = NColors.Dim, modifier = Modifier.padding(start = 8.dp))
            val rows = Remote.BUTTONS.flatMap { (b, bn) ->
                Remote.gesturesOf(b).map { (g, gn) ->
                    @Composable {
                        val a = remember(tick) { Remote.action(context, b, g) }
                        SRow("$bn · $gn", null, value = a.label, chevron = false, onClick = {
                            val all = Remote.Act.values()
                            Remote.setAction(context, b, g, all[(a.ordinal + 1) % all.size]); tick++
                        })
                    }
                }
            }
            SGroup(*rows.toTypedArray())
            Spacer(Modifier.height(8.dp))
            Button(onClick = { Remote.resetActions(context); tick++ }) { Text("Reset shortcuts") }
            Spacer(Modifier.height(24.dp))
        }
    }
}

@Composable
private fun LaunchedEffectOnce(block: () -> Unit) {
    androidx.compose.runtime.LaunchedEffect(Unit) { block() }
}
