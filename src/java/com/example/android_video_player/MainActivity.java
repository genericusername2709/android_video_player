package com.example.android_video_player;

import android.app.AlertDialog;
import android.app.NativeActivity;
import android.content.DialogInterface;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.text.InputType;
import android.util.Log;
import android.view.ViewGroup;
import android.widget.EditText;
import android.widget.FrameLayout;

public class MainActivity extends NativeActivity {
    private static final String TAG = "MainActivity";

    public static volatile MainActivity instance = null;
    public static volatile boolean isPlayerActive = false;
    public static String lastSelectedUri = null;
    public static int lastRequestCode = 0;
    public static boolean hasNewResult = false;
    public static boolean hasPickerFinished = false;

    public static synchronized boolean consumePickerFinished() {
        if (hasPickerFinished) {
            hasPickerFinished = false;
            return true;
        }
        return false;
    }

    public static volatile long lastSelectedPositionMs = 0;
    public static volatile String lastRenamedTitle = null;
    public static volatile int lastRenamedIndex = -1;
    public static volatile boolean hasRenameResult = false;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        instance = this;
        Log.i(TAG, "MainActivity instance initialized in onCreate: " + this);
    }

    @Override
    protected void onDestroy() {
        try {
            if (instance == this) {
                instance = null;
            }
        } catch (Exception e) {
            Log.w(TAG, "Error in onDestroy: " + e.getMessage());
        }
        super.onDestroy();
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        Log.i(TAG, "onActivityResult received: requestCode=" + requestCode + ", resultCode=" + resultCode);

        if (requestCode == 999) {
            if (data != null) {
                long pos = data.getLongExtra("last_position", 0);
                lastSelectedPositionMs = pos;
                Log.i(TAG, "VideoPlayerActivity returned last position: " + pos);
            }
            isPlayerActive = false;
            return;
        }

        synchronized (MainActivity.class) {
            hasPickerFinished = true;
        }

        if (resultCode == RESULT_OK && data != null && data.getData() != null) {
            Uri uri = data.getData();
            Log.i(TAG, "Selected URI from file picker: " + uri.toString());

            try {
                int takeFlags = Intent.FLAG_GRANT_READ_URI_PERMISSION;
                getContentResolver().takePersistableUriPermission(uri, takeFlags);
                Log.i(TAG, "Persistable URI permission granted for: " + uri.toString());
            } catch (Exception e) {
                Log.w(TAG, "Could not take persistable permission: " + e.getMessage());
            }

            synchronized (MainActivity.class) {
                lastSelectedUri = uri.toString();
                lastRequestCode = requestCode;
                hasNewResult = true;
            }
        } else {
            Log.w(TAG, "onActivityResult: cancelled or no data returned");
        }
    }

    public static synchronized String consumeSelectedUri() {
        if (hasNewResult) {
            hasNewResult = false;
            String uri = lastSelectedUri;
            lastSelectedUri = null;
            return uri;
        }
        return null;
    }

    public static synchronized int getLastRequestCode() {
        return lastRequestCode;
    }

    public static void openFilePicker(final int requestCode) {
        Log.i(TAG, "openFilePicker static called with requestCode: " + requestCode);
        MainActivity act = instance;
        if (act == null) {
            Log.e(TAG, "openFilePicker error: MainActivity instance is null");
            return;
        }
        act.runOnUiThread(new OpenFilePickerTask(requestCode));
    }

    private static class OpenFilePickerTask implements Runnable {
        private final int requestCode;

        OpenFilePickerTask(int requestCode) {
            this.requestCode = requestCode;
        }

        @Override
        public void run() {
            MainActivity act = instance;
            if (act == null) return;
            try {
                Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                intent.addCategory(Intent.CATEGORY_OPENABLE);
                intent.setType("*/*");
                String[] mimeTypes = new String[]{"video/*", "audio/*"};
                intent.putExtra(Intent.EXTRA_MIME_TYPES, mimeTypes);
                act.startActivityForResult(intent, requestCode);
                Log.i(TAG, "openFilePicker intent launched successfully on UI thread");
            } catch (Exception e) {
                Log.e(TAG, "Error launching openFilePicker on UI thread", e);
            }
        }
    }

    public static void playMediaInApp(final String uriStr, final long startPositionMs, final String title) {
        Log.i(TAG, "playMediaInApp static called: URI=" + uriStr + ", startPos=" + startPositionMs + ", title=" + title);
        isPlayerActive = true;
        MainActivity act = instance;
        if (act == null || uriStr == null) {
            Log.e(TAG, "playMediaInApp error: MainActivity instance or uriStr is null");
            isPlayerActive = false;
            return;
        }
        act.runOnUiThread(new PlayInAppTask(uriStr, startPositionMs, title));
    }

    private static class PlayInAppTask implements Runnable {
        private final String uriStr;
        private final long startPos;
        private final String title;

        PlayInAppTask(String uriStr, long startPos, String title) {
            this.uriStr = uriStr;
            this.startPos = startPos;
            this.title = title;
        }

        @Override
        public void run() {
            MainActivity act = instance;
            if (act != null) {
                try {
                    Intent intent = new Intent(act, VideoPlayerActivity.class);
                    intent.putExtra("media_uri", uriStr);
                    intent.putExtra("start_position", startPos);
                    intent.putExtra("title", title);
                    act.startActivityForResult(intent, 999);
                    Log.i(TAG, "Launched VideoPlayerActivity for URI: " + uriStr + ", title: " + title);
                } catch (Exception e) {
                    Log.e(TAG, "Error launching VideoPlayerActivity", e);
                    isPlayerActive = false;
                }
            }
        }
    }

    public static boolean isVideoPlayingInApp() {
        return isPlayerActive;
    }

    public static long getPlaybackPosition() {
        return lastSelectedPositionMs;
    }

    public static void showRenameDialog(final int index, final String currentName) {
        Log.i(TAG, "showRenameDialog called for index: " + index + ", currentName: " + currentName);
        MainActivity act = instance;
        if (act == null) return;
        act.runOnUiThread(new ShowRenameDialogTask(index, currentName));
    }

    private static class ShowRenameDialogTask implements Runnable {
        private final int index;
        private final String currentName;

        ShowRenameDialogTask(int index, String currentName) {
            this.index = index;
            this.currentName = currentName;
        }

        @Override
        public void run() {
            MainActivity act = instance;
            if (act != null) {
                act.doShowRenameDialog(index, currentName);
            }
        }
    }

    private void doShowRenameDialog(final int index, final String currentName) {
        try {
            AlertDialog.Builder builder = new AlertDialog.Builder(this);
            builder.setTitle("Rename Favorite");

            final EditText input = new EditText(this);
            input.setInputType(InputType.TYPE_CLASS_TEXT);
            input.setText(currentName != null ? currentName : "");
            input.setSelection(input.getText().length());

            FrameLayout container = new FrameLayout(this);
            FrameLayout.LayoutParams params = new FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
            int margin = (int) (16 * getResources().getDisplayMetrics().density);
            params.setMargins(margin, 0, margin, 0);
            input.setLayoutParams(params);
            container.addView(input);

            builder.setView(container);

            builder.setPositiveButton("Save", new SaveDialogListener(input, index));
            builder.setNegativeButton("Cancel", new CancelDialogListener());

            builder.show();
        } catch (Exception e) {
            Log.e(TAG, "Error showing rename dialog", e);
        }
    }

    private static class SaveDialogListener implements DialogInterface.OnClickListener {
        private final EditText input;
        private final int index;

        SaveDialogListener(EditText input, int index) {
            this.input = input;
            this.index = index;
        }

        @Override
        public void onClick(DialogInterface dialog, int which) {
            String newTitle = input.getText().toString().trim();
            if (!newTitle.isEmpty()) {
                synchronized (MainActivity.class) {
                    lastRenamedTitle = newTitle;
                    lastRenamedIndex = index;
                    hasRenameResult = true;
                }
                Log.i(TAG, "Rename saved: index=" + index + ", newTitle=" + newTitle);
            }
        }
    }

    private static class CancelDialogListener implements DialogInterface.OnClickListener {
        @Override
        public void onClick(DialogInterface dialog, int which) {
            dialog.cancel();
        }
    }

    public static synchronized String consumeRenamedTitle() {
        if (hasRenameResult) {
            return lastRenamedTitle;
        }
        return null;
    }

    public static synchronized int getRenamedIndexAndClear() {
        int idx = lastRenamedIndex;
        hasRenameResult = false;
        lastRenamedTitle = null;
        lastRenamedIndex = -1;
        return idx;
    }

    private static volatile int lastDeletedIndex = -1;
    private static volatile boolean hasDeleteResult = false;

    public static void showDeleteDialog(final int index, final String currentName) {
        Log.i(TAG, "showDeleteDialog called for index: " + index + ", currentName: " + currentName);
        MainActivity act = instance;
        if (act == null) return;
        act.runOnUiThread(new ShowDeleteDialogTask(index, currentName));
    }

    private static class ShowDeleteDialogTask implements Runnable {
        private final int index;
        private final String currentName;

        ShowDeleteDialogTask(int index, String currentName) {
            this.index = index;
            this.currentName = currentName;
        }

        @Override
        public void run() {
            MainActivity act = instance;
            if (act != null) {
                act.doShowDeleteDialog(index, currentName);
            }
        }
    }

    private void doShowDeleteDialog(final int index, final String currentName) {
        try {
            AlertDialog.Builder builder = new AlertDialog.Builder(this);
            builder.setTitle("Delete Favourite");
            builder.setMessage("Are you sure you want to delete '" + (currentName != null ? currentName : "this item") + "' from your favourites?");

            builder.setPositiveButton("Delete", new DeleteClickListener(index));
            builder.setNegativeButton("Cancel", new CancelDialogListener());

            builder.show();
        } catch (Exception e) {
            Log.e(TAG, "Error showing delete dialog", e);
        }
    }

    private static class DeleteClickListener implements DialogInterface.OnClickListener {
        private final int index;

        DeleteClickListener(int index) {
            this.index = index;
        }

        @Override
        public void onClick(DialogInterface dialog, int which) {
            synchronized (MainActivity.class) {
                lastDeletedIndex = index;
                hasDeleteResult = true;
            }
            Log.i(TAG, "Delete confirmed for index: " + index);
        }
    }

    public static synchronized int consumeDeletedIndex() {
        if (hasDeleteResult) {
            int idx = lastDeletedIndex;
            hasDeleteResult = false;
            lastDeletedIndex = -1;
            return idx;
        }
        return -1;
    }
}
