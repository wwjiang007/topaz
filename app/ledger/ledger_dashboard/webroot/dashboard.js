const RECONNECT_INTERVAL = 500;
const MAX_RECONNECT_INTERVAL = 2000;

var _toolbar = null;
var _tabBar = null;
var _webSocket = null;
var _ReqUrl = "http://" + window.location.host + "/data/ledger_debug";
var _reconnectInterval = RECONNECT_INTERVAL;

$(function() {
  mdc.autoInit();
  _toolbar =
    new mdc.toolbar.MDCToolbar(document.querySelector('.mdc-toolbar'));
  _tabBar =
    new mdc.tabs.MDCTabBar(document.querySelector('#dashboard-tab-bar'));

  _tabBar.listen('MDCTabBar:change', function (t) {
      var newPanelSelector = _tabBar.activeTab.root_.hash;
      updateTabPanel(newPanelSelector);
    });
  _tabBar.layout();
});

function updateTabPanel(newPanelSelector) {
  var activePanel = document.querySelector('.panel.active');
  if (activePanel) {
    activePanel.classList.remove('active');
  }
  var newActivePanel = document.querySelector(newPanelSelector);
  if (newActivePanel) {
    newActivePanel.classList.add('active');
  }
}


var app = angular.module('ledgerDashboard', []);
app.controller('debugCtrl', function($scope, $http) {
  $scope.instancesList = [];
  $scope.pagesList = [];
  $scope.showPages = false;
  $scope.selectedInstance = '';
  $scope.bytesToString = function(bytesList) {
    var str = "";
    for(var i = 0; i < bytesList.length; i++) {
      if((bytesList[i] >= 0 && bytesList[i] <= 31)
          || bytesList[i] >= 127) {
        str = bytesToHex(bytesList);
        break;
      }
      str += String.fromCharCode(bytesList[i]);
    }
    return str;
  };

  function bytesToHex (bytes) {
    var hexString = '0x';
    for(var i = 0; i < bytes.length; i++) {
      var temp = bytes[i].toString(16);
      if(temp.length < 2)
        temp = "0" + temp;
      hexString += temp;
    }
    return hexString;
  };

  $scope.getPagesList = function(index) {
    _webSocket.send(JSON.stringify({"instance_name": $scope.instancesList[index]}));
  };

  function connectWebSocket() {
    $scope.showPages = false;
    _webSocket = new WebSocket("ws://" + window.location.host + "/ws/ledger_debug/");
    _webSocket.onopen = handleWebSocketOpen;
    _webSocket.onerror = handleWebSocketError;
    _webSocket.onclose = handleWebSocketClose;
    _webSocket.onmessage = handleWebSocketMessage;
  }

  function handleWebSocketOpen(evt) {
    $("#connectedLabel").text("Connected");
    // reset reconnect
    _reconnectInterval = RECONNECT_INTERVAL;
  }

  function handleWebSocketError(evt) {
    console.log("WebSocket Error: " + evt.toString());
  }

  function handleWebSocketClose(evt) {
    $("#connectedLabel").text("Disconnected");
    // attempt to reconnect
    attemptReconnect();
  }

  function attemptReconnect() {
    console.log("Attempting to reconnect after " + _reconnectInterval);

    // reconnect after the timeout
    setTimeout(connectWebSocket, _reconnectInterval);
    // exponential reconnect timeout
    var nextInterval = _reconnectInterval * 2;
    if (nextInterval < MAX_RECONNECT_INTERVAL) {
      _reconnectInterval = nextInterval;
    } else {
      _reconnectInterval = MAX_RECONNECT_INTERVAL;
    }
  }

  function handleWebSocketMessage(evt) {
    // parse the JSON message
    var message = JSON.parse(evt.data);
    if ("instances_list" in message) {
      $scope.instancesList = message["instances_list"];
    }
    if ("pages_list" in message) {
      $scope.showPages = true;
      $scope.pagesList = message["pages_list"];
    }
    $scope.$apply();
  }

  $(document).ready(function(){
    connectWebSocket();
  });

});
