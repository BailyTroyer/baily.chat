import { render } from "solid-js/web";
import { getMainDefinition } from "@apollo/client/utilities";
import {
  ApolloClient,
  InMemoryCache,
  HttpLink,
  split,
} from "@apollo/client/core";
import { GraphQLWsLink } from "@apollo/client/link/subscriptions";
import { createClient } from "graphql-ws";

import { ApolloProvider } from "./apollo";
import { Messages } from "./messages";

import "./index.css";

const httpLink = new HttpLink({
  uri: "http://44.202.103.102:8000/",
  headers: {
    "Content-Type": "application/json",
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Credentials": "true",
  },
});

const wsLink = new GraphQLWsLink(
  createClient({
    url: "ws://44.202.103.102:8000/ws",
  })
);

const splitLink = split(
  ({ query }) => {
    const definition = getMainDefinition(query);
    return (
      definition.kind === "OperationDefinition" &&
      definition.operation === "subscription"
    );
  },
  wsLink,
  httpLink
);

const client = new ApolloClient({
  cache: new InMemoryCache(),
  link: splitLink,
  // headers: {
  //   "Content-Type": "application/json",
  //   "Access-Control-Allow-Origin": "*",
  //   "Access-Control-Allow-Methods": "*",
  //   "Access-Control-Allow-Headers": "true",
  //   "Access-Control-Allow-Credentials": "true",
  // },
});

render(
  () => (
    <ApolloProvider client={client}>
      <Messages />
    </ApolloProvider>
  ),
  document.getElementById("root") as HTMLElement
);
