package com.govorun.lite.ui

import android.content.Context
import android.content.pm.PackageManager
import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import androidx.appcompat.app.AppCompatActivity
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.google.android.material.appbar.MaterialToolbar
import com.google.android.material.button.MaterialButtonToggleGroup
import com.google.android.material.checkbox.MaterialCheckBox
import com.google.android.material.color.DynamicColors
import com.google.android.material.textfield.TextInputEditText
import com.google.android.material.textview.MaterialTextView
import com.govorun.lite.R
import com.govorun.lite.util.Prefs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class AppFilterActivity : AppCompatActivity() {

    data class AppEntry(
        val label: String,
        val packageName: String,
        var checked: Boolean
    )

    private lateinit var recyclerView: RecyclerView
    private lateinit var filterModeToggle: MaterialButtonToggleGroup
    private lateinit var searchInput: TextInputEditText
    private lateinit var filterDesc: MaterialTextView

    private val allApps = mutableListOf<AppEntry>()
    private var filteredApps = listOf<AppEntry>()
    private lateinit var adapter: AppListAdapter

    override fun onCreate(savedInstanceState: Bundle?) {
        DynamicColors.applyToActivityIfAvailable(this)
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_app_filter)

        val toolbar = findViewById<MaterialToolbar>(R.id.toolbar)
        setSupportActionBar(toolbar)
        supportActionBar?.setDisplayHomeAsUpEnabled(true)

        recyclerView     = findViewById(R.id.app_list)
        filterModeToggle = findViewById(R.id.filter_mode_toggle)
        searchInput      = findViewById(R.id.search_input)
        filterDesc       = findViewById(R.id.filterDesc)

        val savedMode = Prefs.getAppFilterMode(this)
        val savedPkgs = Prefs.getAppFilterPackages(this)

        filterModeToggle.check(
            if (savedMode == Prefs.APP_FILTER_WHITELIST) R.id.btn_whitelist else R.id.btn_blacklist
        )
        updateDesc(savedMode)
        filterModeToggle.addOnButtonCheckedListener { _, checkedId, isChecked ->
            if (!isChecked) return@addOnButtonCheckedListener
            val mode = if (checkedId == R.id.btn_whitelist) Prefs.APP_FILTER_WHITELIST
                       else Prefs.APP_FILTER_BLACKLIST
            updateDesc(mode)
            savePrefs()
        }

        searchInput.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {}
            override fun afterTextChanged(s: Editable?) { applySearch(s?.toString() ?: "") }
        })

        adapter = AppListAdapter()
        recyclerView.layoutManager = LinearLayoutManager(this)
        recyclerView.adapter = adapter

        CoroutineScope(Dispatchers.Main).launch {
            val apps = withContext(Dispatchers.IO) { loadApps(savedPkgs) }
            allApps.clear()
            allApps.addAll(apps)
            reSort()
        }
    }

    override fun onSupportNavigateUp(): Boolean { finish(); return true }

    private fun updateDesc(mode: String) {
        filterDesc.setText(
            if (mode == Prefs.APP_FILTER_WHITELIST) R.string.app_filter_desc_whitelist
            else R.string.app_filter_desc_blacklist
        )
    }

    private fun loadApps(savedPkgs: Set<String>): List<AppEntry> {
        val pm = packageManager
        val intent = android.content.Intent(android.content.Intent.ACTION_MAIN, null).apply {
            addCategory(android.content.Intent.CATEGORY_LAUNCHER)
        }
        return pm.queryIntentActivities(intent, 0)
            .map { it.activityInfo.packageName }
            .distinct()
            .filter { it != packageName }  // exclude ourselves
            .map { pkg ->
                val label = try {
                    pm.getApplicationLabel(pm.getApplicationInfo(pkg, 0)).toString()
                } catch (_: PackageManager.NameNotFoundException) { pkg }
                AppEntry(label = label, packageName = pkg, checked = pkg in savedPkgs)
            }
    }

    private fun reSort() {
        allApps.sortWith(compareByDescending<AppEntry> { it.checked }.thenBy { it.label.lowercase() })
        applySearch(searchInput.text?.toString() ?: "")
    }

    private fun applySearch(query: String) {
        filteredApps = if (query.isBlank()) allApps.toList()
        else allApps.filter {
            it.label.contains(query, ignoreCase = true) ||
            it.packageName.contains(query, ignoreCase = true)
        }
        adapter.notifyDataSetChanged()
    }

    private fun savePrefs() {
        val mode    = if (filterModeToggle.checkedButtonId == R.id.btn_whitelist)
            Prefs.APP_FILTER_WHITELIST else Prefs.APP_FILTER_BLACKLIST
        val checked = allApps.filter { it.checked }.map { it.packageName }.toSet()
        Prefs.setAppFilterMode(this, mode)
        Prefs.setAppFilterPackages(this, checked)
    }

    inner class AppListAdapter : RecyclerView.Adapter<AppListAdapter.ViewHolder>() {

        inner class ViewHolder(view: View) : RecyclerView.ViewHolder(view) {
            val icon: ImageView           = view.findViewById(R.id.app_icon)
            val name: MaterialTextView    = view.findViewById(R.id.app_name)
            val pkg: MaterialTextView     = view.findViewById(R.id.app_package)
            val checkbox: MaterialCheckBox = view.findViewById(R.id.app_checkbox)
        }

        override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
            val view = LayoutInflater.from(parent.context)
                .inflate(R.layout.list_item_app, parent, false)
            return ViewHolder(view)
        }

        override fun onBindViewHolder(holder: ViewHolder, position: Int) {
            val entry = filteredApps[position]
            holder.name.text = entry.label
            holder.pkg.text  = entry.packageName
            holder.checkbox.isChecked = entry.checked

            try {
                holder.icon.setImageDrawable(packageManager.getApplicationIcon(entry.packageName))
            } catch (_: Exception) {}

            holder.itemView.setOnClickListener {
                entry.checked = !entry.checked
                savePrefs()
                reSort()
            }
        }

        override fun getItemCount() = filteredApps.size
    }
}
