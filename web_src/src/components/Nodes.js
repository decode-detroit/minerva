import React from 'react';

// An action list element
export class Action extends React.PureComponent {
  constructor(props) {
    // Collect props and set initial state
    super(props);

    // Create a node reference
    this.node = React.createRef();

    // Bind the various functions
    /*this.handleMouseDown = this.handleMouseDown.bind(this);
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseClose = this.handleMouseClose.bind(this);*/
  }
  
  // Create any connectors on load
  componentDidMount() {
    // Identify all the connections
    let connections = [];
    // Switch based on the props
    if (this.props.action.hasOwnProperty(`NewScene`)) {
      connections.push({type: "scene", id: this.props.action.NewScene.new_scene.id});
    }

    // Create each connection
    connections.forEach((connection) => {
      this.props.createConnector(connection.type, this.node.current, connection.id);
    });
  }

  // Render the event action
  render() {
    // Switch based on the props
    if (this.props.action.hasOwnProperty(`NewScene`)) {
      return (
        <div className="action">
          New Scene
          <SendNode ref={this.node} type="scene"></SendNode>
        </div>
      );
    
    // Modify Status
    } else if (this.props.action.hasOwnProperty(`ModifyStatus`)) {
      return (
        <div className="action">
          Modify Status
          <SendNode ref={this.node} type="status"></SendNode>
        </div>
      );
    
    // Cue Event
    } else if (this.props.action.hasOwnProperty(`CueEvent`)) {
      return (
        <div className="action">
          Cue Event
          <SendNode ref={this.node} type="event"></SendNode>
        </div>
      );
    
    // Cancel Event
    } else if (this.props.action.hasOwnProperty(`CancelEvent`)) {
      return (
        <div className="action">
          Cancel Event
          <SendNode ref={this.node} type="event"></SendNode>
        </div>
      );
    
    // Save Data
    } else if (this.props.action.hasOwnProperty(`SaveData`)) {
      return (
        <div className="action">
          Save Data
        </div>
      );
    
    // Send Data
    } else if (this.props.action.hasOwnProperty(`SendData`)) {
      return (
        <div className="action">
          Send Data
        </div>
      );

    // Select Event
    } else if (this.props.action.hasOwnProperty(`SelectEvent`)) {
      return (
        <div className="action">
          Select Event
          <SendNode ref={this.node} type="status"></SendNode>
        </div>
      );
    }
    
    // Otherwise, return the default
    return (
        <div className="action">Invalid Action</div>
    );
  }
}

// A receive Node element
export class ReceiveNode extends React.PureComponent {  
  // Render the completed link
  render() {
    return (
      <div className={`node ${this.props.type}`} ></div>
    );
  }
}

// A send Node element
export class SendNode extends React.PureComponent {
  // Render the completed link
  render() {
    return (
      <div className={`node ${this.props.type}`} ></div>
    );
  }
}