import React from 'react';
import { Action, ReceiveNode } from './Nodes';
import { stopPropogation } from './functions';

// An item box to select the appropriate sub-box
export class ItemBox extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      left: 0, // horizontal offset of the area
      top: 0, // vertical offest of the area
      cursorX: 0, // starting point of the cursor
      cursorY: 0,
      itemPair: { // placeholder for the real data
        id: 0,
        description: "Loading ...",
      },
      type: "",
    }

    // The timeout to save changes, if set
    this.saveTimeout = null;

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
    this.handleMouseDown = this.handleMouseDown.bind(this);
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseClose = this.handleMouseClose.bind(this);
    this.handleChange = this.handleChange.bind(this);
  }
  
  // Function to respond to clicking the area
  handleMouseDown(e) {
    stopPropogation();
   
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
      left = (left >= 0) ? left : 0;
      top = (top >= 0) ? top : 0;
  
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

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.item.isValid) {
        this.setState({
          itemPair: json.item.itemPair,
        });
      }

      // Check to see if the item is a scene
      response = await fetch(`getType/${this.props.id}`);
      const json2 = await response.json();

      // If valid, save the result to the state
      if (json2.generic.isValid) {
        this.setState({
          type: json2.generic.message,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // On initial load, pull the location and item information
  componentDidMount() {
    // Set the starting location
    this.setState({
      left: this.props.left,
      top: this.props.top,
    })

    // Pull the item information
    this.updateItem();
  }

  // Function to handle new text in the input
  handleChange(e) {
    // Extract the value
    let value = e.target.value;

    // Replace the existing description
    this.setState(prevState => ({
      itemPair: {...prevState.itemPair, description: value},
    }));

    // CLear the existing timeout, if it exists
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
    }

    // Save the changes after a second pause
    let editItem = {
      modifications: [{
        modifyItem: {
          itemPair: {...this.state.itemPair, description: value},
        },
      }],
    };
    this.saveTimeout = setTimeout(async () => {
      const response = await fetch(`/edit`, {
          method: 'POST',
          headers: {
              'Content-Type': 'application/json',
          },
          body: JSON.stringify(editItem),
      });
      const json = await response.json(); //extract JSON from the http response
      console.log(json); // FIXME
    }, 1000);
  }
 
  // Return the selected box
  render() {
    // Return the item box
    return (
      <>
        {this.state.type !== "" &&
          <div className={`box ${this.state.type} ${this.props.isFocus ? 'focus' : ''}`} style={{ left: `${this.state.left}px`, top: `${this.state.top}px` }} onMouseDown={(e) => {stopPropogation(e); this.props.grabFocus(this.props.id)}}>
            <div className="title">
              <input type="text" value={this.state.itemPair.description} size={this.state.itemPair.description.length > 30 ? this.state.itemPair.description.length - 10 : 20} onInput={this.handleChange}></input>
              <div>({this.state.itemPair.id})</div>
            </div>
            <ReceiveNode id={`receive-node-${this.state.itemPair.id}`} type={this.state.type} onMouseDown={this.handleMouseDown}/>
            {this.props.isFocus && this.state.type === "scene" && <SceneFragment id={this.props.id} changeScene={this.props.changeScene}/>}
            {this.props.isFocus && this.state.type === "status" && <StatusFragment id={this.props.id} grabFocus={this.props.grabFocus} createConnector={this.props.createConnector}/>}
            {this.props.isFocus && this.state.type === "event" && <EventFragment id={this.props.id} grabFocus={this.props.grabFocus} createConnector={this.props.createConnector}/>}
          </div>
        }
      </>
    );
  }
}

// A scene box with an scene and scene detail
export class SceneFragment extends React.PureComponent {
  // Return the fragment
  render() {
    return (
      <div className="divButton" onClick={() => {this.props.changeScene(this.props.id)}}>View This Scene</div>
    );
  }
}

// A statue box with a status and status detail
export class StatusFragment extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      status: {},
    }

    // Bind the various functions
    this.updateStatus = this.updateStatus.bind(this);
  }

  // Helper function to update the status information
  async updateStatus() {
    try {
      // Fetch the detail of the status
      const response = await fetch(`getStatus/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.status.isValid) {
        this.setState({
          status: json.status.status,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // On initial load, pull the status information
  componentDidMount() {
    // Pull the new status information
    this.updateStatus();
  }
 
  // Return the fragment
  render() {
    // Compose any actions into a list
    //const children = this.state.eventActions.map((action) => <Action key={action.toString()} action={action} createConnector={this.props.createConnector}></Action>);

    // If the status is a multistate

    // Return the fragment
    return (
      <>
        <div className="subtitle">States:</div>
        <div className="verticalList"></div>
      </>
    );
  }
}

// An event box with an event and event detail
export class EventFragment extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      eventActions: [], // placeholder for the read data
    }

    // Bind the various functions
    this.updateEvent = this.updateEvent.bind(this);
  }

  // Helper function to update the event information
  async updateEvent() {
    try {
      // Fetch the detail of the event
      const response = await fetch(`getEvent/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.event.isValid) {
        this.setState({
          eventActions: json.event.event,
        });
      }
    
    // Ignore errors
    } catch {
      console.log(`Server inaccessible: ${this.props.id}`); // FIXME
    }
  }

  // Helper function to change an event action FIXME
  changeAction(index, action) {    
    // Save the new state
    this.setState((prevState) => {
      let newActions = prevState.eventActions;
      newActions[index] = action;

      console.log("New" + eventActions)
      return {
        eventActions: newActions,
      };
    });

    // FIXME send the change to the server
  }

  // On initial load, pull the event information
  componentDidMount() {
    // Pull the new event information
    this.updateEvent();
  }

  // Return the fragment
  render() {
    // Compose any actions into a list
    console.log(this.state.eventActions);
    const children = this.state.eventActions.map((action, index) => <Action key={action.toString()} action={action} grabFocus={this.props.grabFocus} changeAction={(newAction) => {this.changeAction(index, newAction)}} createConnector={this.props.createConnector}></Action>);

    // Return the fragment
    return (
      <>
        <div className="subtitle">Actions:</div>
        <div className="verticalList">{children}</div>
      </>
    );
  }
}

