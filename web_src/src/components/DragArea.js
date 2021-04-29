import React from 'react';
import { EventDialog, SmallDialog } from './Dialogs';
import { Link } from './TextComponents';

// A box to contain the draggable edit area
export class ViewBox extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);
    this.state = {
      itemList: [], // list of all configuration items
      cursorX: 0, // starting point of the cursor
      cursorY: 0,
      isMenuVisible: false, // flag to show the context menu
    }

    // Bind the various functions
    this.handleMouseDown = this.handleMouseDown.bind(this);
    this.showContextMenu = this.showContextMenu.bind(this);
  }
  
  // Function to respond to clicking the area
  handleMouseDown(e) {
    // Hide the menu
    this.setState({
      isMenuVisible: false,
    });
  }

  // Function to show the context menu at the correct location
  showContextMenu(e) {
    // Prevent the default event handler
    e = e || window.event;
    e.preventDefault();

    // Update the cusor location and mark the menu as visible
    this.setState({
      cursorX: e.clientX,
      cursorY: e.clientY,
      isMenuVisible: true,
    });

    return false;
  }

  // On render, pull the full item list
  componentDidUpdate() {
    fetch(`/allItems`)
    .then(response => {
      return response.json()
    })
    .then(json => {
      console.log(json);

      // Save the list to the state
      this.setState({
        list: json.items
      })
    });
  }
  
  // Render the edit area inside the viewbox
  render() {
    return (
      <div className="viewBox" onContextMenu={this.showContextMenu} onMouseDown={this.handleMouseDown}>
        <EditArea></EditArea>
        {this.state.isMenuVisible && <SmallDialog left={this.state.cursorX} top={this.state.cursorY - 30 - vmin(1)} title={"Add Item"} children={[<Link text={"New Event"}></Link>, <Link text={"Existing Event"}></Link>, <Link text={"New Status"}></Link>,  <Link text={"Existing Status"}></Link>,  <Link text={"New Scene"}></Link>]}></SmallDialog>}
      </div>
    );
  }
}

// The draggable edit area
export class EditArea extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);
    this.state = {
      left: 0, // horizontal offset of the area
      top: 0, // vertical offest of the area
      cursorX: 0, // starting point of the cursor
      cursorY: 0,
    }

    // Bind the various functions
    this.handleMouseDown = this.handleMouseDown.bind(this);
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseClose = this.handleMouseClose.bind(this);
  }

  // Function to respond to clicking the area
  handleMouseDown(e) {
    // Prevent any other event handlers
    e = e || window.event;
    e.preventDefault();
   
    // Connect the mouse event handlers to the document
    document.onmousemove = this.handleMouseMove;
    document.onmouseup = this.handleMouseClose;

    // Save the cursor position, hide the menu
    this.setState({
      cursorX: e.clientX,
      cursorY: e.clientY,
    });
  }

  // Function to respond to dragging the area
  handleMouseMove(e) {
    // Prevent the default event handler
    e = e || window.event;
    e.preventDefault();

    // Update the state
    this.setState((state) => {
      // Calculate change from old cursor position
      let changeX = state.cursorX - e.clientX;
      let changeY = state.cursorY - e.clientY;
  
      // Calculate the new location
      let left = state.left - changeX;
      let top = state.top - changeY;
  
      // Enforce bounds on the new location
      left = (left <= 0) ? left : 0;
      top = (top <= 0) ? top : 0;
  
      // Save the new location and current cursor position
      return {
        left: left,
        top: top,
        cursorX: e.clientX,
        cursorY: e.clientY,
      }
    });
  }
  
  // Function to respond to releasing the mouse
  handleMouseClose() {
    // Stop moving when mouse button is released
    document.onmousemove = null;
    document.onmouseup = null;
  }

  // Render the draggable edit area
  render() {
    return (
        <div className="editArea" style={{ left: `${this.state.left}px`, top: `${this.state.top}px` }} onMouseDown={this.handleMouseDown}>
          <EventDialog id={101} left={300} top={300}></EventDialog>
        </div>
    )
  }
}


// Helper functions for calculating dialog offset
function vh(v) {
  var h = Math.max(document.documentElement.clientHeight, window.innerHeight || 0);
  return (v * h) / 100;
}

function vw(v) {
  var w = Math.max(document.documentElement.clientWidth, window.innerWidth || 0);
  return (v * w) / 100;
}

function vmin(v) {
  return Math.min(vh(v), vw(v));
}
