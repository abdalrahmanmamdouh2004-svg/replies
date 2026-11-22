package com.example

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** Find the watch and connect. Nothing else. */
@Composable
fun WatchScreen(onBack: () -> Unit) {
    val context = LocalContext.current
    val status by WatchLinkState.status.collectAsState()
    val connected by WatchLinkState.connected.collectAsState()
    val scanning by WatchLinkState.scanning.collectAsState()
    val devices by WatchLinkState.devices.collectAsState()
    val savedName by WatchLinkState.savedName.collectAsState()
    val ringing by FindPhone.ringing.collectAsState()

    var slowTip by remember { mutableStateOf(false) }
    LaunchedEffect(scanning) {
        slowTip = false
        if (scanning) {
            kotlinx.coroutines.delay(45_000)
            slowTip = true
        }
    }

    LaunchedEffect(Unit) { WatchLink.initialize(context) }

    LaunchedEffect(connected) {
        if (connected && WatchFeatureSwitches.isEnabled(context, WatchFeature.WEATHER)) {
            WeatherSync.onConnected(context)
        }
    }

    val permissionRequest = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) {
        if (WatchLink.hasPermission()) WatchLink.startScan()
        else WatchLinkState.status.value = "Bluetooth permission denied"
    }

    Column(Modifier.fillMaxSize()) {
        NTopBar(title = "Watch", onBack = onBack)
        Column(
            Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            NCard {
                Text(status, color = if (connected) NColors.Green else NColors.Text, fontFamily = NType, fontSize = 13.sp)
                Spacer(Modifier.height(8.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    NButton(
                        text = if (scanning) "Stop" else "Find watch",
                        onClick = {
                            if (scanning) WatchLink.stopScan()
                            else if (WatchLink.hasPermission()) WatchLink.startScan()
                            else permissionRequest.launch(WatchLink.neededPermissions())
                        },
                        primary = !connected
                    )
                    NButton("Disconnect", { WatchLink.disconnect(forget = false) })
                }
                if (scanning && slowTip) {
                    Spacer(Modifier.height(10.dp))
                    Text("Turn on Power Saver mode, then turn it off.", color = NColors.Text, fontFamily = NDot, fontSize = 13.sp)
                }
            }

            devices.forEach { (name, address) ->
                NCard {
                    Text(name, color = NColors.Text, fontFamily = NDot, fontSize = 18.sp)
                    Spacer(Modifier.height(6.dp))
                    NButton(
                        text = if (connected && savedName == name) "Connected" else "Connect",
                        onClick = { MediaMenuService.connectWatch(context, address, name) },
                        enabled = !connected,
                        primary = name.contains("CMF Watch Pro 2", true)
                    )
                }
            }

            if (ringing) {
                NCard {
                    Text("Phone is ringing", color = NColors.Text, fontFamily = NType, fontSize = 13.sp)
                    Spacer(Modifier.height(6.dp))
                    NButton("Stop ringing", { FindPhone.stop(context) }, primary = true)
                }
            }
        }
    }
}
