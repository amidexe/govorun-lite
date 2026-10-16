package com.govorun.lite.transcriber

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/** Read immediately while VAD loads. Bounded FIFO preserves the oldest sound;
 * processing cannot delay microphone reads or reuse a mutable input frame. */
@OptIn(ExperimentalCoroutinesApi::class)
internal class BufferedPcmCapture(
    scope: CoroutineScope,
    windowBytes: Int,
    keepReading: () -> Boolean,
    read: (ByteArray, Int, Int) -> Int,
) {
    val frames = Channel<ByteArray>(Channel.BUFFERED)
    val job = scope.launch(Dispatchers.IO, start = CoroutineStart.ATOMIC) {
        try {
            while (isActive && keepReading()) {
                val frame = ByteArray(windowBytes)
                var offset = 0
                while (offset < frame.size && isActive && keepReading()) {
                    val count = read(frame, offset, frame.size - offset)
                    if (count <= 0) break
                    offset += count
                }
                if (offset > 0) frames.send(if (offset == frame.size) frame else frame.copyOf(offset))
                if (offset < frame.size) break
            }
        } finally { frames.close() }
    }
}
