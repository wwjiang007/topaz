// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'dart:core';

import 'package:fidl_fuchsia_documents/fidl.dart' as doc_fidl;
import 'package:flutter/material.dart';
import 'package:meta/meta.dart';

/// An item that can be selected, mult-selected, and interacted with
abstract class SelectableItem extends StatelessWidget {
  /// Document attached to this item view
  final doc_fidl.Document doc;

  /// True if this document is currently selected or multi-selected
  final bool selected;

  /// Function to call when item is pressed
  final VoidCallback onPressed;

  /// Function to call when item is double tapped
  final VoidCallback onDoubleTap;

  /// Function to call when item is pressed
  final VoidCallback onLongPress;

  /// Whether to show the checkbox for multi-select
  final bool hideCheckbox;

  /// Constructor
  const SelectableItem({
    @required this.doc,
    @required this.selected,
    Key key,
    this.onPressed,
    this.onDoubleTap,
    this.onLongPress,
    this.hideCheckbox,
  })  : assert(doc != null),
        assert(selected != null),
        super(key: key);
}
