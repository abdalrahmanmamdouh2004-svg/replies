package com.example

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35])
class RepliesUtf8Test {
    @Test
    fun arabicNamesAndMessagesStayUtf8AtEveryByteLimit() {
        val sample = "أحمد: مرحباً بالعالم"
        val bytes = sample.toByteArray(Charsets.UTF_8)
        for (limit in 0..(bytes.size + 2)) {
            val result = Replies.fitUtf8(sample, limit)
            val encoded = result.toByteArray(Charsets.UTF_8)
            assertTrue("result exceeds $limit bytes", encoded.size <= limit)
            assertEquals(result, String(encoded, Charsets.UTF_8))
            assertFalse("Arabic was replaced by dots", result.contains('·'))
        }
    }

    @Test
    fun truncationNeverLeavesHalfAnEmojiOrArabicCharacter() {
        val sample = "أحمد😀رسالة"
        val bytes = sample.toByteArray(Charsets.UTF_8)
        for (limit in 0..bytes.size) {
            val result = Replies.fitUtf8(sample, limit)
            assertEquals(result, String(result.toByteArray(Charsets.UTF_8), Charsets.UTF_8))
        }
    }
}
