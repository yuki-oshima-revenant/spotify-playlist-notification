#!/usr/bin/env node
import * as cdk from "aws-cdk-lib";
import { CdkStack } from "../lib/cdk-stack";

const app = new cdk.App();
new CdkStack(app, "SpotifyPlaylistNotificationStack", {
    env: {
        account: "621702102095",
        region: "ap-northeast-1",
    },
});
