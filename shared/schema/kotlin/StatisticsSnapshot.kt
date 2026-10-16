// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class StatisticsSnapshot(
    val `days`: List<StatisticsDay>,
    val `hasCloudSnapshot`: Boolean,
    val `signedIn`: Boolean,
    val `storageReadOnly`: Boolean,
    val `storageRecovered`: Boolean,
    val `syncMode`: StatisticsMode,
    val `today`: String,
    val `total`: StatisticsTotal
)
