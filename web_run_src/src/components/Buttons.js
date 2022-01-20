import React from 'react';
import { stopPropogation } from './Functions';

// A confirm button to confirm selection before proceeding
export class ConfirmButton extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      isSelected: false,
      isConfirmed: false,
      buttonHeight: null,
      buttonWidth: null,
    };

    // Bind the various functions
    this.handleClick = this.handleClick.bind(this);

    // Create a reference for the element
    this.button = React.createRef();

    // Create a holder for the timeout
    this.timeout = null;
  }
  
  // Function to respond to clicking the area
  handleClick(e) {
    stopPropogation(e);

    // If the button is already selected
    if (this.state.isSelected) {
      // Trigger the specified callback
      this.props.onClick();

      // Mark as triggered and remove selected
      this.setState({
        isSelected: false,
        isConfirmed: true,
      });

      // Clear any existing timeout
      if (this.timeout) {
        clearTimeout(this.timeout);
      }

      // Set the timeout to return to normal
      this.timeout = setTimeout(() => {
        // Reset the state and remove the timeout
        this.setState({
          isSelected: false,
          isConfirmed: false,
        })
        this.timeout = null;
      }, 1000);
    
    // Otherwise, select the button
    } else {
      // Mark as selected
      this.setState({
        isSelected: true,
        isConfirmed: false,
      });

      // Clear any existing timeout
      if (this.timeout) {
        clearTimeout(this.timeout);
      }

      // Set the timeout to return to normal
      this.timeout = setTimeout(() => {
        // Reset the state and remove the timeout
        this.setState({
          isSelected: false,
          isConfirmed: false,
        })
        this.timeout = null;
      }, 1000);
    }
  }

  // On initial load, lock the button size
  componentDidMount() {
    this.setState({
      buttonHeight: this.button.current.offsetHeight,
      buttonWidth: this.button.current.offsetWidth,
    });
  }
 
  // Return the selected box
  render() {
    // Return the item box
    return (
      <button className={`${this.props.buttonClass}` + (this.state.isSelected ? " red" : "") + (this.state.isConfirmed ? " green" : "")} ref={this.button} style={{ height: `${this.state.buttonHeight}px`, width: `${this.state.buttonWidth}px` }} onClick={this.handleClick}>
        {!this.state.isSelected && `${this.props.buttonText}`}
        {this.state.isSelected && "Confirm?"}
      </button>
    );
  }
}

// An item button to  cue the selected item
export class ItemButton extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      description: "Loading ...", // placeholder for the real data
      display: {},
      type: "",
    };

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
    this.handleMouseDown = this.handleMouseDown.bind(this);
  }
  
  // Function to respond to clicking the area
  handleMouseDown(e) {
    stopPropogation(e);

    // Trigger the selected event FIXME should be moved to event subtype
    let cueEvent = {
      id: this.props.id,
      secs: 0,
      nanos: 0,
    };
    fetch(`/cueEvent`, {
      method: 'POST',
      headers: {
          'Content-Type': 'application/json',
      },
      body: JSON.stringify(cueEvent),
    }); // FIXME ignore errors
  }

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.item.isValid) {
        // Save the itemPair
        this.setState({
          description: json.item.itemPair.description,
          display: json.item.itemPair.display,
        });
      }

      // Check to see the item type
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
    // Pull the item information
    this.updateItem();
  }
 
  // Return the selected box
  render() {
    // Return the item box
    return (
      <>
        {this.state.type !== "" &&
          <div id={`id-${this.props.id}`} className={`box ${this.state.type} row${this.props.row} ${this.props.isFocus ? 'focus' : ''}`} onMouseDown={this.handleMouseDown}>
            <div className="title">
              <div>{this.state.description}</div>
              <div className="disableSelect">({this.props.id})</div>
            </div>
          </div>
        }
      </>
    );
  }
}

// An empty box with no type
/*export class BlankFragment extends React.PureComponent {
  // Return the fragment
  render() {
    return (
      <>
        <div className="subtitle">Choose Item Type</div>
        <div className="typeChooser">
          <div className="divButton event" onClick={() => {let modifications = [{ modifyEvent: { itemId: { id: this.props.id }, event: [], }}]; this.props.saveModifications(modifications); this.props.updateItem()}}>Event</div>
          <div className="divButton status" onClick={() => {let modifications = [{ modifyStatus: { itemId: { id: this.props.id }, status: { MultiState: { current: { id: 0 }, allowed: [], no_change_silent: false, }}}}]; this.props.saveModifications(modifications); this.props.updateItem()}}>Status</div>
          <div className="divButton scene" onClick={() => {let modifications = [{ modifyScene: { itemId: { id: this.props.id }, scene: { events: [], }}}]; this.props.saveModifications(modifications); this.props.updateItem()}}>Scene</div>
        </div>
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
      isAddMenuVisible: "",
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

  // Helper function to add a new state
  addState(id) {
    if (this.state.status.hasOwnProperty(`MultiState`)) {
      // Save the new state id
      this.setState((prevState) => {
        // Compose the new status
        let newStatus = { MultiState:
          {...prevState.status.MultiState, allowed: [...prevState.status.MultiState.allowed, {id: id}]}
        };

        // Save the changes
        let modifications = [{
          modifyStatus: {
            itemId: { id: this.props.id },
            status: newStatus,
          },
        }];
        this.props.saveModifications(modifications);

        // Update the local state
        return {
          status: newStatus,
        };
      });
    
    // Ignore types other than multistate
    } else {
      console.error("This feature not yet implemented.");
    }
  }

  // Helper function to remove a state
  removeState(index) {
    if (this.state.status.hasOwnProperty(`MultiState`)) {
      // Save the new state id
      this.setState((prevState) => {
        // Remove the state from the allowed list
        let newAllowed = [...prevState.status.MultiState.allowed];
        newAllowed.splice(index, 1);

        // Compose the new status
        let newStatus = { MultiState:
          {...prevState.status.MultiState, allowed: newAllowed}
        };

        // Save the changes
        let modifications = [{
          modifyStatus: {
            itemId: { id: this.props.id },
            status: newStatus,
          },
        }];
        this.props.saveModifications(modifications);

        // Update the local state
        return {
          status: newStatus,
        };
      });
    
    // Ignore types other than multistate
    } else {
      console.error("This feature not yet implemented.");
    }
  }

  // On initial load, pull the status information
  componentDidMount() {
    // Pull the new status information
    this.updateStatus();
  }

  // Return the fragment
  render() {
    // Compose any states into a list
    let children = [];
    if (this.state.status.hasOwnProperty(`MultiState`)) {
      children = this.state.status.MultiState.allowed.map((state, index) => <State key={state.toString()} state={state} grabFocus={this.props.grabFocus} removeState={() => {this.removeState(index)}} createConnector={this.props.createConnector} />);
    }

    // Return the fragment
    return (
      <>
        <div className="subtitle">States:</div>
        <div className="verticalList" onWheel={stopPropogation}>{children}
          {this.state.selectMenu}
        </div>
        <div className="addButton" onClick={() => {this.setState(prevState => ({ isAddMenuVisible: !prevState.isAddMenuVisible }))}}>
          {this.state.isAddMenuVisible ? `-` : `+`}
          {this.state.isAddMenuVisible && <AddMenu type="event" left={180} top={60} addItem={(id) => {this.setState({ isAddMenuVisible: false }); this.addState(id)}} saveModifications={this.props.saveModifications}/>}
        </div>
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
      isAddMenuVisible: false,
      selectMenu: null,
    }

    // Bind the various functions
    this.updateEvent = this.updateEvent.bind(this);
    this.addAction = this.addAction.bind(this);
    this.changeAction = this.changeAction.bind(this);
    this.selectMenu = this.selectMenu.bind(this);
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
          eventActions: json.event.event.actions,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Helper function to add a new event action
  addAction(action) {
    // Save the change action
    this.setState((prevState) => {
      // Copy the existing action list and add a blank action
      let newActions = [...prevState.eventActions, action];

      // Save the changes
      let modifications = [{
        modifyEvent: {
          itemId: { id: this.props.id },
          event: newActions,
        },
      }];
      this.props.saveModifications(modifications);

      // Update the local state
      return {
        eventActions: [...newActions],
        isAddMenuVisible: false,
      };
    });
  }

  // Helper function to change an event action
  changeAction(index, action) {
    // Save the change action
    this.setState((prevState) => {
      // Copy the existing action list
      let newActions = [...prevState.eventActions];

      // If an action was specified, replace it
      if (action) {
        newActions[index] = action;

      // Otherwise, remove that index number
      } else {
        newActions.splice(index, 1);
      }

      // Save the changes
      let modifications = [{
        modifyWebEvent: {
          itemId: { id: this.props.id },
          event: {
            actions: newActions,
          }
        },
      }];
      this.props.saveModifications(modifications);

      // Update the local state
      return {
        eventActions: [...newActions],
      };
    });
  }

  // Helper function to save the new select menu
  selectMenu(newMenu) {
    // Check to see if it's already set
    if (newMenu !== null && this.state.selectMenu !== null) {
      return false;
    }
    
    // Otherwise, update it and return true
    this.setState({
      selectMenu: newMenu,
    })
    return true;
  }

  // On initial load, pull the event information
  componentDidMount() {
    // Pull the new event information
    this.updateEvent();
  }

  // Return the fragment
  render() {
    // Compose any actions into a list
    const children = this.state.eventActions.map((action, index) => <Action key={action.toString()} action={action} grabFocus={this.props.grabFocus} changeAction={(newAction) => {this.changeAction(index, newAction)}} selectMenu={this.selectMenu} createConnector={this.props.createConnector}/>);

    // Return the fragment
    return (
      <>
        <div className="subtitle">Actions:</div>
        <div className="verticalList" onWheel={stopPropogation}>
          {children}
          {this.state.selectMenu}
        </div>
        <div className="addButton" onClick={() => {this.setState(prevState => ({ isAddMenuVisible: !prevState.isAddMenuVisible }))}}>
          {this.state.isAddMenuVisible ? `-` : `+`}
          {this.state.isAddMenuVisible && <AddActionMenu left={180} top={60} addAction={this.addAction}/>}
        </div>
      </>
    );
  }
}
*/
