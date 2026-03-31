# THIS FILE IS AUTO-GENERATED. DO NOT MODIFY!!

# Copyright 2020-2023 Tauri Programme within The Commons Conservancy
# SPDX-License-Identifier: Apache-2.0
# SPDX-License-Identifier: MIT

-keep class com.sprklai.zenii.mobile.* {
  native <methods>;
}

-keep class com.sprklai.zenii.mobile.WryActivity {
  public <init>(...);

  void setWebView(com.sprklai.zenii.mobile.RustWebView);
  java.lang.Class getAppClass(...);
  java.lang.String getVersion();
}

-keep class com.sprklai.zenii.mobile.Ipc {
  public <init>(...);

  @android.webkit.JavascriptInterface public <methods>;
}

-keep class com.sprklai.zenii.mobile.RustWebView {
  public <init>(...);

  void loadUrlMainThread(...);
  void loadHTMLMainThread(...);
  void evalScript(...);
}

-keep class com.sprklai.zenii.mobile.RustWebChromeClient,com.sprklai.zenii.mobile.RustWebViewClient {
  public <init>(...);
}
