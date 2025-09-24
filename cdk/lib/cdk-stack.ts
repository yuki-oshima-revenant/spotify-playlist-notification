import { join } from "path";
import {
    aws_dynamodb,
    aws_iam,
    aws_scheduler,
    aws_scheduler_targets,
    Duration,
    Stack,
    TimeZone,
    type StackProps,
} from "aws-cdk-lib";
import { ServicePrincipal } from "aws-cdk-lib/aws-iam";
import type { Construct } from "constructs";
import { RustFunction } from "cargo-lambda-cdk";
import { Architecture } from "aws-cdk-lib/aws-lambda";

export class CdkStack extends Stack {
    constructor(scope: Construct, id: string, props?: StackProps) {
        super(scope, id, props);
        const localTestUser = new aws_iam.User(this, "LocalTestUser", {
            userName: "spotify-playlist-notification-local-test",
        });
        const role = new aws_iam.Role(this, "LambdaRole", {
            assumedBy: new ServicePrincipal("lambda.amazonaws.com"),
        });
        role.addManagedPolicy({
            managedPolicyArn:
                "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole",
        });

        // todo: secrets managerから環境変数を取得できるようにする
        const lambda = new RustFunction(this, "Lambda", {
            role,
            manifestPath: join(__dirname, "..", "..", "backend"),
            architecture: Architecture.ARM_64,
            timeout: Duration.minutes(5),
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
        userTable.grantReadData(lambda);

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
        spotifyRefreshTokenTable.grantReadData(lambda);
        spotifyRefreshTokenTable.grantWriteData(lambda);

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
        lastNotifiedTrackTable.grantReadData(lambda);
        lastNotifiedTrackTable.grantWriteData(lambda);

        new aws_scheduler.Schedule(this, "Schedule", {
            schedule: aws_scheduler.ScheduleExpression.cron({
                minute: "0",
                hour: "12",
                day: "*",
                month: "*",
                year: "*",
                timeZone: TimeZone.ASIA_TOKYO,
            }),
            target: new aws_scheduler_targets.LambdaInvoke(lambda),
        });
    }
}
