package com.officewala.tmediaplayer;

import android.app.Activity;
import android.content.Intent;
import android.graphics.Typeface;
import android.media.MediaPlayer;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.FrameLayout;
import android.widget.MediaController;
import android.widget.VideoView;

public class VideoPlayerActivity extends Activity {
  private static final String TAG = "VideoPlayerActivity";

  public static volatile VideoPlayerActivity activeInstance = null;

  private VideoView videoView;
  private MediaController mediaController;
  private Button closeBtn;
  private long startPositionMs = 0;
  private volatile long lastValidPositionMs = 0;
  private volatile boolean isCompleted = false;

  private final Handler positionHandler = new Handler(Looper.getMainLooper());
  private final PositionTrackerRunnable positionTracker = new PositionTrackerRunnable();

  private static class PositionTrackerRunnable implements Runnable {
    @Override
    public void run() {
      VideoPlayerActivity act = activeInstance;
      if (act != null && act.videoView != null && !act.isCompleted) {
        try {
          if (act.videoView.isPlaying()) {
            int pos = act.videoView.getCurrentPosition();
            if (pos > 0) {
              act.lastValidPositionMs = pos;
            }
          }
        } catch (Exception ignored) {
        }
        act.positionHandler.postDelayed(this, 200);
      }
    }
  }

  @Override
  protected void onCreate(Bundle savedInstanceState) {
    super.onCreate(savedInstanceState);
    activeInstance = this;

    Intent intent = getIntent();
    String uriStr = intent != null ? intent.getStringExtra("media_uri") : null;
    startPositionMs = intent != null ? intent.getLongExtra("start_position", 0) : 0;
    String title = intent != null ? intent.getStringExtra("title") : null;
    
    lastValidPositionMs = startPositionMs;
    isCompleted = false;

    if (uriStr == null) {
      Log.e(TAG, "No media_uri provided to VideoPlayerActivity");
      finish();
      return;
    }

    if (title != null && !title.trim().isEmpty()) {
      setTitle(title);
    }

    FrameLayout rootLayout = new FrameLayout(this);
    rootLayout.setBackgroundColor(0xFF000000);

    videoView = new VideoView(this);
    FrameLayout.LayoutParams videoParams = new FrameLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.MATCH_PARENT,
        Gravity.CENTER);
    rootLayout.addView(videoView, videoParams);

    mediaController = new MediaController(this);
    mediaController.setAnchorView(videoView);
    videoView.setMediaController(mediaController);

    float density = getResources().getDisplayMetrics().density;
    int displayH = getResources().getDisplayMetrics().heightPixels;

    // Floating Close Button
    closeBtn = new Button(this);
    closeBtn.setText("X");
    closeBtn.setTextSize(TypedValue.COMPLEX_UNIT_SP, 16);
    closeBtn.setTypeface(null, Typeface.BOLD);
    closeBtn.setTextColor(0xFFFFFFFF);
    closeBtn.setBackgroundColor(0xCCDD0000);
    closeBtn.setElevation(1000f);

    int padH = (int) (14 * density);
    int padV = (int) (8 * density);
    closeBtn.setPadding(padH, padV, padH, padV);

    FrameLayout.LayoutParams closeParams = new FrameLayout.LayoutParams(
        ViewGroup.LayoutParams.WRAP_CONTENT,
        ViewGroup.LayoutParams.WRAP_CONTENT,
        Gravity.TOP | Gravity.END
    );
    int marginTop = (int) (displayH * 0.05); // Clears camera cutout
    int marginH = (int) (16 * density);
    closeParams.setMargins(0, marginTop, marginH, 0);
    closeBtn.setLayoutParams(closeParams);
    closeBtn.setOnClickListener(new CloseClickListener());
    rootLayout.addView(closeBtn);

    setContentView(rootLayout);

    Uri uri = Uri.parse(uriStr);
    videoView.setOnPreparedListener(new OnPreparedListenerImpl(startPositionMs));
    videoView.setOnCompletionListener(new OnCompletionListenerImpl());
    videoView.setVideoURI(uri);

    videoView.requestFocus();
    positionHandler.post(positionTracker);
  }

  @Override
  protected void onDestroy() {
    positionHandler.removeCallbacks(positionTracker);
    if (activeInstance == this) {
      activeInstance = null;
    }
    super.onDestroy();
  }

  private static class CloseClickListener implements View.OnClickListener {
    @Override
    public void onClick(View v) {
      VideoPlayerActivity act = activeInstance;
      if (act != null) {
        act.closeAndFinish();
      }
    }
  }

  private static class OnPreparedListenerImpl implements MediaPlayer.OnPreparedListener {
    private final long startPos;

    OnPreparedListenerImpl(long startPos) {
      this.startPos = startPos;
    }

    @Override
    public void onPrepared(MediaPlayer mp) {
      VideoPlayerActivity act = activeInstance;
      if (act != null && act.videoView != null) {
        if (startPos > 0) {
          act.videoView.seekTo((int) startPos);
        }
        act.videoView.start();
        if (act.mediaController != null) {
          act.mediaController.show(3000);
        }
      }
    }
  }

  private static class OnCompletionListenerImpl implements MediaPlayer.OnCompletionListener {
    @Override
    public void onCompletion(MediaPlayer mp) {
      VideoPlayerActivity act = activeInstance;
      if (act != null) {
        act.isCompleted = true;
        act.lastValidPositionMs = 0;
        act.closeAndFinish();
      }
    }
  }

  @Override
  public void onBackPressed() {
    closeAndFinish();
  }

  @Override
  public boolean onKeyDown(int keyCode, KeyEvent event) {
    if (keyCode == KeyEvent.KEYCODE_BACK) {
      closeAndFinish();
      return true;
    }
    return super.onKeyDown(keyCode, event);
  }

  public void closeAndFinish() {
    if (videoView != null) {
      try {
        if (videoView.isPlaying()) {
          int currentPos = videoView.getCurrentPosition();
          if (currentPos > 0) {
            lastValidPositionMs = currentPos;
          }
          videoView.stopPlayback();
        }
      } catch (Exception ignored) {
      }
    }

    Log.i(TAG, "closeAndFinish returning last_position: " + lastValidPositionMs);
    Intent resultIntent = new Intent();
    resultIntent.putExtra("last_position", lastValidPositionMs);
    resultIntent.putExtra("final_position", lastValidPositionMs);
    setResult(RESULT_OK, resultIntent);
    finish();
  }
}
