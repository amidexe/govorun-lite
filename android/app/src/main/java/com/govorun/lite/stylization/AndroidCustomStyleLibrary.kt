package com.govorun.lite.stylization

import com.govorun.lite.model.stylization.CustomStylePreset
import com.govorun.lite.model.stylization.StyleOptions
import com.govorun.lite.shared.SharedBusiness
import com.govorun.lite.shared.SharedBusinessException
import org.json.JSONObject

/** Storage/UUID ports only. Rules and the persistence decision are owned by Rust. */
internal interface CustomStyleStorage {
    val lock: Any
    fun read(): JSONObject
    fun apply(before: JSONObject, after: JSONObject)
    fun selectedOptions(): StyleOptions
    fun newId(): String
}

internal class AndroidCustomStyleLibrary(
    private val storage: CustomStyleStorage,
    private val beforeChange: () -> Unit,
    private val transaction: ((() -> JSONObject?) -> JSONObject?) = { it() },
) {
    fun snapshot(): JSONObject? = synchronized(storage.lock) {
        try { call(JSONObject().put("kind", "project").put("state", storage.read())) }
        catch (_: IllegalArgumentException) { null }
    }
    fun list(): List<CustomStylePreset> = snapshot()?.getJSONObject("state")
        ?.getJSONArray("presets")?.let(CustomStyleBinding::presets).orEmpty()
    fun activeId(): String? = snapshot()?.getJSONObject("projection")?.let {
        if (it.isNull("activeId")) null else it.getString("activeId")
    }
    fun select(id: String): Boolean = mutate(JSONObject().put("kind", "select").put("id", id)) != null
    fun saveAndSelect(id: String?, name: String, instruction: String): CustomStylePreset? =
        saveInLibrary(id, name, instruction, storage.selectedOptions(), selectSaved = true)
    fun saveInLibrary(id: String?, name: String, instruction: String,
        editedOptions: StyleOptions? = null, selectSaved: Boolean = false): CustomStylePreset? = mutate(JSONObject().put("kind", "save")
            .put("id", id ?: JSONObject.NULL).put("newId", storage.newId())
            .put("name", name).put("instruction", instruction).put("selectSaved", selectSaved)
            .put("options", editedOptions?.let(CustomStyleBinding::options) ?: JSONObject.NULL))?.let(::saved)
    fun saveActiveInstruction(instruction: String): CustomStylePreset? = mutate(JSONObject().put("kind", "saveActive")
        .put("newId", storage.newId()).put("instruction", instruction)
        .put("currentOptions", CustomStyleBinding.options(storage.selectedOptions())))?.let(::saved)
    fun delete(id: String) { mutate(JSONObject().put("kind", "delete").put("id", id)) }
    fun updateOptions(id: String, options: StyleOptions) {
        mutate(JSONObject().put("kind", "options").put("id", id).put("options", CustomStyleBinding.options(options)))
    }
    fun replaceFromSync(presets: List<CustomStylePreset>, selectedID: String) {
        check(mutate(JSONObject().put("kind", "replace").put("presets", CustomStyleBinding.encode(presets))
            .put("selectedId", selectedID)) != null) { "style_storage_unavailable" }
    }

    private fun saved(frame: JSONObject): CustomStylePreset? = frame.optJSONObject("preset")?.let(CustomStyleBinding::preset)
    private fun mutate(command: JSONObject): JSONObject? = transaction {
        synchronized(storage.lock) {
            val original = try { storage.read() } catch (_: IllegalArgumentException) { return@synchronized null }
            val frame = try { call(command.put("state", original)) }
                catch (error: SharedBusinessException) {
                    if (error.code != "account_invalid_request") throw error
                    return@synchronized null
                }
            if (!frame.getBoolean("accepted")) return@synchronized null
            if (frame.getBoolean("write")) {
                if (frame.getBoolean("markDirty")) beforeChange()
                storage.apply(original, frame.getJSONObject("state"))
            }
            frame
        }
    }
    private fun call(command: JSONObject): JSONObject = SharedBusiness.evaluate(
        JSONObject().put("operation", "styleLibrary").put("command", command))
}
