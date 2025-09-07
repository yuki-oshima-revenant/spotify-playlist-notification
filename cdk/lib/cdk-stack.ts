import { aws_dynamodb, aws_iam, Stack, StackProps } from "aws-cdk-lib";
import { Construct } from "constructs";

export class CdkStack extends Stack {
    constructor(scope: Construct, id: string, props?: StackProps) {
        super(scope, id, props);
        const localTestUser = new aws_iam.User(this, "LocalTestUser", {
            userName: "spotify-playlist-notification-local-test",
        });

        const userTable = new aws_dynamodb.TableV2(this, "UserTable", {
            tableName: "spotify-playlist-notification_user",
            partitionKey: {
                name: "name",
                type: aws_dynamodb.AttributeType.STRING,
            },
            sortKey: {
                name: "order",
                type: aws_dynamodb.AttributeType.NUMBER,
            },
        });
        userTable.grantReadData(localTestUser);

        const spotifyRefreshTokenTable = new aws_dynamodb.TableV2(
            this,
            "SpotifyRefreshTokenTable",
            {
                tableName:
                    "spotify-playlist-notification_spotify_refresh_token",
                partitionKey: {
                    name: "singleton_key",
                    type: aws_dynamodb.AttributeType.STRING,
                },
            },
        );
        spotifyRefreshTokenTable.grantReadData(localTestUser);
        spotifyRefreshTokenTable.grantWriteData(localTestUser);

        const lastNotifiedTrackTable = new aws_dynamodb.TableV2(
            this,
            "LastNotifiedTrackTable",
            {
                tableName: "spotify-playlist-notification_last_notified_track",
                partitionKey: {
                    name: "singleton_key",
                    type: aws_dynamodb.AttributeType.STRING,
                },
            },
        );
        lastNotifiedTrackTable.grantReadData(localTestUser);
        lastNotifiedTrackTable.grantWriteData(localTestUser);
    }
}
