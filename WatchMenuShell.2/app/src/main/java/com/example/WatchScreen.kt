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
import kotlinx.coroutines.delay

/** Connects WatchMenu to the watch after Nothing X has established the phone's BLE connection. */
@Composable
fun WatchScreen(onBack: () -> Unit) {
    val context = LocalContext.current
    val status by WatchLinkState.status.collectAsState()
    val connected by WatchLinkState.connected.collectAsState()
    val scanning by WatchLinkState.scanning.collectAsState()
    val devices by WatchLinkState.devices.collectAsState()
    val savedName by WatchLinkState.savedName.collectAsState()
    val ringing by FindPhone.ringing.collectAsState()

    var sharedMode by remember { mutableStateOf(FmcSharedConnection.isSelected(context)) }
    var nothingXDevices by remember { mutableStateOf<List<Pair<String, String>>>(emptyList()) }
    var showResults by remember { mutableStateOf(false) }

    val permissionRequest = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) {
        if (WatchLink.hasPermission()) {
            nothingXDevices = FmcSharedConnection.connectedNothingXWatches(context)
        } else {
            WatchLinkState.status.value = "Bluetooth permission denied"
        }
    }

    LaunchedEffect(Unit) {
        WatchLink.initialize(context)
        if (!WatchLink.hasPermission()) {
            permissionRequest.launch(WatchLink.neededPermissions())
        }
        while (true) {
            nothingXDevices = if (WatchLink.hasPermission()) {
                FmcSharedConnection.connectedNothingXWatches(context)
            } else {
                emptyList()
            }
            delay(1000)
        }
    }

    val nothingXDetected = nothingXDevices.isNotEmpty() || FmcSharedConnection.isConnected()
    // Direct mode remains available for the native alarm/legacy commands.
    val connectionPrerequisiteMet = !sharedMode || nothingXDetected
    val displayDevices = remember(devices, nothingXDevices) {
        (nothingXDevices + devices).distinctBy { it.second }
    }

    Column(Modifier.fillMaxSize()) {
        NTopBar(title = "Connect Watch", onBack = onBack)
        Column(
            Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            NCard {
                SSwitchRow(
                    label = "Connect through Nothing X",
                    checked = sharedMode,
                    onChange = { enabled ->
                        if (enabled != sharedMode) {
                            WatchLink.disconnect(forget = false)
                            FmcSharedConnection.setSelected(context, enabled)
                            sharedMode = enabled
                            WatchLinkState.status.value = "Not connected"
                            showResults = false
                        }
                    }
                )
                Text(
                    "Log in to Nothing X and connect your CMF Watch Pro 2 to the app.",
                    color = NColors.Dim,
                    fontFamily = NType,
                    fontSize = 12.sp
                )
                Spacer(Modifier.height(8.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    NButton(
                        text = if (scanning) "Stop" else "Find watch",
                        onClick = {
                            if (scanning) {
                                WatchLink.stopScan()
                            } else if (WatchLink.hasPermission()) {
                                showResults = true
                                WatchLink.startScan()
                            } else {
                                permissionRequest.launch(WatchLink.neededPermissions())
                            }
                        },
                        enabled = connectionPrerequisiteMet,
                        primary = connectionPrerequisiteMet && !connected
                    )
                    NButton(
                        "Disconnect",
                        onClick = { WatchLink.disconnect(forget = false) },
                        enabled = connectionPrerequisiteMet
                    )
                }
                if (connected) {
                    Spacer(Modifier.height(6.dp))
                    Text(
                        "CMF Watch Pro 2 is connected",
                        color = NColors.Green,
                        fontFamily = NType,
                        fontSize = 13.sp
                    )
                } else if (status == "Connection failed") {
                    Spacer(Modifier.height(6.dp))
                    Text("Connection failed", color = NColors.Red, fontFamily = NType, fontSize = 13.sp)
                }
            }

            if (showResults) {
                displayDevices.forEach { (name, address) ->
                    NCard {
                        Text(name, color = NColors.Text, fontFamily = NDot, fontSize = 18.sp)
                        Spacer(Modifier.height(6.dp))
                        NButton(
                            text = if (connected && savedName == name) "Connected" else "Connect",
                            onClick = { MediaMenuService.connectWatch(context, address, name) },
                            enabled = !connected && connectionPrerequisiteMet,
                            primary = name.contains("CMF Watch Pro 2", true)
                        )
                    }
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
